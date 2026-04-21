use core::{cell::RefCell, mem, ops::Range};

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    sync::Arc,
};

use crate::{
    fd::{self, FileDescriptor, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO},
    io::IoInterface,
    kernel,
    memory::{
        self, Frame, VmSpace,
        paging::{
            L1, L2, L3, L4, Level, PhysicalAddress, PhysicalPageTable, TableLevel, VirtualAddress,
        },
    },
    scheduler,
};

pub const STACK_SIZE: usize = 0x3000;
pub const INTERRUPT_STACK_SIZE: usize = 0x3000;
pub const NUM_PRIORITIES: usize = 32;
pub const REALTIME_PRIORITY: TaskPriority = TaskPriority(NUM_PRIORITIES as u8 - 1);
pub const HIGH_PRIORITY: TaskPriority = TaskPriority(24);
pub const NORMAL_PRIORITY: TaskPriority = TaskPriority(16);
pub const LOW_PRIORITY: TaskPriority = TaskPriority(0);

#[repr(C, packed)]
struct State {
    gs: u64,
    fs: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rsp: u64,
    rbx: u64,
    rdx: u64,
    rcx: u64,
    rax: u64,
    rflags: u64,
    rip: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TaskStatus {
    Invalid,
    Ready,
    Running,
    Blocked,
    Finished,
    Idle,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskId(u64);

impl TaskId {
    pub const fn new(id: u64) -> Self {
        TaskId(id)
    }

    pub const fn into(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for TaskId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct TaskPriority(u8);

impl TaskPriority {
    pub const fn new(priority: u8) -> Self {
        TaskPriority(priority)
    }

    pub const fn into(self) -> u8 {
        self.0
    }
}

pub(crate) struct PriorityTaskQueue {
    queues: [VecDeque<Rc<RefCell<Task>>>; NUM_PRIORITIES],
    priority_bitmap: usize,
}

impl PriorityTaskQueue {
    pub const fn new() -> PriorityTaskQueue {
        Self {
            queues: [const { VecDeque::new() }; NUM_PRIORITIES],
            priority_bitmap: 0,
        }
    }

    pub fn push(&mut self, task: &Rc<RefCell<Task>>) {
        let priority = task.borrow().priority.into() as usize;
        self.priority_bitmap |= 1 << priority;
        self.queues[priority].push_back(Rc::clone(task));
    }

    fn pop_from_queue(&mut self, queue_index: usize) -> Option<Rc<RefCell<Task>>> {
        let task = self.queues[queue_index].pop_front();
        if self.queues[queue_index].is_empty() {
            self.priority_bitmap &= !(1 << queue_index);
        }

        task
    }

    /// Pop the next task, which has a higher or the same priority as `priority`.
    pub fn pop_with_priority(&mut self, priority: TaskPriority) -> Option<Rc<RefCell<Task>>> {
        if self.priority_bitmap == 0 {
            return None;
        }

        let p = self.priority_bitmap.trailing_zeros() as usize;
        if p >= priority.into() as usize {
            return self.pop_from_queue(p);
        }

        None
    }

    pub fn pop(&mut self) -> Option<Rc<RefCell<Task>>> {
        if self.priority_bitmap == 0 {
            return None;
        }

        let highest_priority = self.priority_bitmap.trailing_zeros() as usize;
        self.pop_with_priority(TaskPriority(highest_priority as u8))
    }
}

#[allow(dead_code)]
pub(crate) trait Stack {
    fn top(&self) -> VirtualAddress;
    fn bottom(&self) -> VirtualAddress;
    fn interrupt_top(&self) -> VirtualAddress;
    fn interrupt_bottom(&self) -> VirtualAddress;
}

pub(crate) struct TaskStack {
    buffer: [u8; STACK_SIZE],
    ist_buffer: [u8; INTERRUPT_STACK_SIZE],
}

#[repr(align(64))]
pub struct Task {
    pub id: TaskId,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub last_stack_ptr: usize,
    pub stack: Box<dyn Stack>,

    /// The physical address of PML4 page table for this task
    pub root_page_table: PhysicalAddress,
    pub fd_map: BTreeMap<FileDescriptor, Arc<dyn IoInterface>>,
    pub vmspace: VmSpace,
}

impl Task {
    pub fn new(id: TaskId, status: TaskStatus, priority: TaskPriority) -> Self {
        let mut fd_map: BTreeMap<FileDescriptor, Arc<dyn IoInterface>> = BTreeMap::new();

        fd_map.insert(STDIN_FILENO, Arc::new(fd::generic::GenericStdin));
        fd_map.insert(STDOUT_FILENO, Arc::new(fd::generic::GenericStdout));
        fd_map.insert(STDERR_FILENO, Arc::new(fd::generic::GenericStderr));

        Self {
            id,
            status,
            priority,
            last_stack_ptr: 0,
            stack: Box::new(TaskStack::new()),
            root_page_table: kernel::get_kernel_page_table().get_physical_address(),
            fd_map,
            vmspace: VmSpace::new(),
        }
    }

    pub fn new_idle(id: TaskId) -> Self {
        Self {
            id,
            status: TaskStatus::Idle,
            priority: LOW_PRIORITY,
            last_stack_ptr: 0,
            stack: Box::new(TaskStack::new()),
            root_page_table: kernel::get_kernel_page_table().get_physical_address(),
            vmspace: VmSpace::new(),
            fd_map: BTreeMap::new(),
        }
    }

    pub fn create_stack_frame(&mut self, entry_point: extern "C" fn()) {
        let mut sp: *mut u64 = self.stack.top().as_mut_ptr();
        unsafe {
            // stack poisoning
            core::ptr::write_bytes(self.stack.bottom().as_mut_ptr::<u8>(), 0xCD, STACK_SIZE);
            sp = sp.offset(-1);
            *sp = leave_task as *const () as _;

            let sp_before = sp;
            // reserve space for state
            sp = (sp as usize - mem::size_of::<State>()) as _;

            let state: *mut State = sp as *mut State;

            // zero out the state memory
            core::ptr::write_bytes(state, 0x0, 1);

            (*state).rsp = sp_before as _;
            (*state).rbp = (*state).rsp + mem::size_of::<u64>() as u64;
            (*state).gs = self.stack.top().0;
            (*state).rip = (entry_point as *const ()) as _;
            (*state).rflags = 0x1202;

            self.last_stack_ptr = sp as usize;
        }
    }
}

impl Default for TaskStack {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskStack {
    pub const fn new() -> TaskStack {
        TaskStack {
            buffer: [0; STACK_SIZE],
            ist_buffer: [0; INTERRUPT_STACK_SIZE],
        }
    }
}

impl Stack for TaskStack {
    fn top(&self) -> VirtualAddress {
        VirtualAddress((self.buffer.as_ptr() as usize + STACK_SIZE - 16) as _)
    }

    fn bottom(&self) -> VirtualAddress {
        VirtualAddress(self.buffer.as_ptr() as _)
    }

    fn interrupt_top(&self) -> VirtualAddress {
        VirtualAddress((self.ist_buffer.as_ptr() as usize + INTERRUPT_STACK_SIZE - 16) as _)
    }

    fn interrupt_bottom(&self) -> VirtualAddress {
        VirtualAddress(self.ist_buffer.as_ptr() as _)
    }
}

pub fn leave_task() {
    scheduler::exit();
}

trait DroppableRegion {
    const DROPPABLE_RANGE: Range<usize> = 0..512;
}

impl DroppableRegion for L3 {}
impl DroppableRegion for L2 {}
impl DroppableRegion for L1 {}
impl DroppableRegion for L4 {
    /// entires: 256..512 refers to the kernel space
    const DROPPABLE_RANGE: Range<usize> = 0..256;
}

trait RecursiveDrop<L: Level> {
    fn recursive_drop(ptr: *mut PhysicalPageTable<L>);
}

impl<L> RecursiveDrop<L> for L
where
    L: TableLevel + Level + DroppableRegion,
    L::NextLevel: RecursiveDrop<L::NextLevel>,
{
    fn recursive_drop(ptr: *mut PhysicalPageTable<L>) {
        let hhdm_offset = kernel::get_hhdm_offset();
        let physical_frame = Frame::from_addr((ptr as usize - hhdm_offset) as _);

        for entry in unsafe { &(&*ptr)[L::DROPPABLE_RANGE] } {
            if entry.is_present() {
                let next_table_ptr: *mut PhysicalPageTable<L::NextLevel> =
                    (entry.get_physical_address().0 as usize + hhdm_offset) as _;
                <L as TableLevel>::NextLevel::recursive_drop(next_table_ptr);
            }
        }

        memory::deallocate_frame(physical_frame);
    }
}

impl RecursiveDrop<L1> for L1 {
    fn recursive_drop(ptr: *mut PhysicalPageTable<L1>) {
        log::trace!("Dealloc L1");
        memory::deallocate_frame(Frame::from_addr(ptr as usize - kernel::get_hhdm_offset()));
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if self.root_page_table != kernel::get_kernel_page_table().get_physical_address() {
            log::debug!("Deallocating page table of task id: {}", self.id);

            let hhdm_offset = kernel::get_hhdm_offset();
            let ptr: *mut PhysicalPageTable<L4> =
                (self.root_page_table.0 as usize + hhdm_offset) as _;
            L4::recursive_drop(ptr);
        }
    }
}
