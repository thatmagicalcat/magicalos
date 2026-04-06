use core::{cell::RefCell, mem};

use alloc::{boxed::Box, collections::VecDeque, rc::Rc};

use crate::memory::paging::VirtualAddress;

pub const STACK_SIZE: usize = 0x3000;
pub const INTERRUPT_STACK_SIZE: usize = 0x3000;
pub const NUM_PRIORITIES: usize = 32;
pub const REALTIME_PRIORITY: TaskPriority = TaskPriority(NUM_PRIORITIES as u8 - 1);
pub const HIGH_PRIORITY: TaskPriority = TaskPriority(24);
pub const NORMAL_PRIORITY: TaskPriority = TaskPriority(16);
pub const LOW_PRIORITY: TaskPriority = TaskPriority(0);

#[repr(C, packed)]
struct State {
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
pub(crate) enum TaskStatus {
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
pub(crate) struct Task {
    pub id: TaskId,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub last_stack_ptr: usize,
    pub stack: Box<dyn Stack>,
}

impl Task {
    pub fn new(id: TaskId, status: TaskStatus, priority: TaskPriority) -> Self {
        Self {
            id,
            status,
            priority,
            last_stack_ptr: 0,
            stack: Box::new(TaskStack::new()),
        }
    }

    pub fn new_idle(id: TaskId) -> Self {
        Self {
            id,
            status: TaskStatus::Idle,
            priority: LOW_PRIORITY,
            last_stack_ptr: 0,
            stack: Box::new(TaskStack::new()),
        }
    }

    pub fn create_stack_frame(&mut self, entry_point: extern "C" fn()) {
        let mut sp: *mut u64 = self.stack.top().as_mut_ptr();
        unsafe {
            sp = sp.offset(-2);

            // this procedure cleans the task after exit
            *sp = leave_task as *const () as _;

            // reserve space for state
            let sp_before = sp;
            sp = (sp as usize - mem::size_of::<State>()) as _;

            let state: *mut State = sp as _;
            (*state).rip = sp_before as _;
            (*state).rflags = 0x1202u64;
            (*state).rip = entry_point as *const () as _;

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
    unimplemented!("implement syscalls bruh")
}
