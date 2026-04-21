use core::{
    cell::RefCell,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    sync::Arc,
};

use crate::{
    arch::interrupts,
    fd::FileDescriptor,
    io::{self, IoInterface},
    memory::paging::{PhysicalAddress, VirtualAddress},
    scheduler::task::{NUM_PRIORITIES, TaskStatus},
    utils,
};

use super::task::{PriorityTaskQueue, Task, TaskId, TaskPriority};

static TASKID_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Virtual address of the last used FpuState
static FPU_OWNER: AtomicUsize = AtomicUsize::new(0);

type RcTask = Rc<RefCell<Task>>;

pub(crate) struct Scheduler {
    current_task: RcTask,
    idle_task: RcTask,
    ready_queue: PriorityTaskQueue,
    finished_tasks: VecDeque<TaskId>,
    tasks: BTreeMap<TaskId, RcTask>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        let task_id = TaskId::new(TASKID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let idle_task = Rc::new(RefCell::new(Task::new_idle(task_id)));
        let mut tasks = BTreeMap::new();

        tasks.insert(task_id, Rc::clone(&idle_task));

        Self {
            current_task: Rc::clone(&idle_task),
            idle_task,
            ready_queue: PriorityTaskQueue::new(),
            finished_tasks: VecDeque::new(),
            tasks,
        }
    }

    pub(crate) fn handle_fpu_fault(&self) {
        log::trace!("Saving/Loading FPU state");

        interrupts::without_interrupts(|| {
            let current_fpu_state_ptr =
                &raw mut self.current_task.borrow_mut().fpu_state.0 as usize;
            let last_fpu_owner_state_ptr = FPU_OWNER.load(Ordering::Relaxed);

            unsafe { core::arch::asm!("clts", options(nomem, nostack, preserves_flags)) };

            // should rarely  happens but good to be safe :)
            if last_fpu_owner_state_ptr == current_fpu_state_ptr {
                return;
            }

            if last_fpu_owner_state_ptr != 0 {
                // the values in the registers are owned by the previous FPU owner
                unsafe {
                    core::arch::asm! {
                        "fxsave [{}]",
                        in(reg) last_fpu_owner_state_ptr,
                        options(nostack, preserves_flags)
                    };
                }
            }

            // restore the FPU state of current task
            unsafe {
                core::arch::asm! {
                    "fxrstor [{}]",
                    in(reg) current_fpu_state_ptr,
                    options(nostack, preserves_flags)
                };
            }

            // current task is the new owner
            FPU_OWNER.store(current_fpu_state_ptr, Ordering::Relaxed);
        });
    }

    fn new_task_id(&self) -> TaskId {
        TaskId::new(TASKID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    pub fn get_current_interrupt_stack(&self) -> VirtualAddress {
        interrupts::without_interrupts(|| self.current_task.borrow().stack.interrupt_top())
    }

    pub fn spawn<F>(&mut self, f: F, priority: TaskPriority) -> Result<TaskId, &'static str>
    where
        F: FnOnce() + Send + 'static,
    {
        interrupts::without_interrupts(|| {
            let priority_no = priority.into() as usize;

            if priority_no >= NUM_PRIORITIES {
                return Err("Priority must be between 0 and NUM_PRIORITIES - 1");
            }

            let task_id = self.new_task_id();
            let task = Rc::new(RefCell::new(Task::new(
                task_id,
                TaskStatus::Ready,
                priority,
            )));

            extern "C" fn trampoline<F: FnOnce() + Send + 'static>(closure_ptr: usize) {
                let closure: F = unsafe { core::ptr::read(closure_ptr as _) };
                closure();
            }

            let closure_ptr = task.borrow_mut().push_onto_stack(f);

            task.borrow_mut()
                .create_stack_frame(trampoline::<F> as *const () as _, closure_ptr);

            self.ready_queue.push(&task);
            self.tasks.insert(task_id, Rc::clone(&task));

            log::info!(
                "Spawned task with ID {:?} and priority {:?}",
                task_id,
                priority,
            );

            Ok(task_id)
        })
    }

    pub(crate) fn with_current_task<T, F>(&self, f: F) -> T
    where
        F: for<'a> FnOnce(&'a mut Task) -> T,
    {
        interrupts::without_interrupts(|| {
            let mut task = self.current_task.borrow_mut();
            f(&mut task)
        })
    }

    pub(crate) fn get_io_interface(&self, fd: FileDescriptor) -> io::Result<Arc<dyn IoInterface>> {
        interrupts::without_interrupts(|| {
            self.current_task
                .borrow()
                .fd_map
                .get(&fd)
                .map(Arc::clone)
                .ok_or(io::Error::NoSuchFileOrDirectory)
        })
    }

    pub(crate) fn add_io_interface(
        &self,
        interface: Arc<dyn IoInterface>,
    ) -> io::Result<FileDescriptor> {
        // find a free file descriptor
        let fd = (0..FileDescriptor::MAX)
            .find(|i| !self.current_task.borrow().fd_map.contains_key(i))
            .ok_or(io::Error::TooManyOpenFiles)?;

        interrupts::without_interrupts(|| {
            self.current_task.borrow_mut().fd_map.insert(fd, interface);
        });

        Ok(fd)
    }

    pub fn remove_io_interface(&self, fd: FileDescriptor) -> io::Result<Arc<dyn IoInterface>> {
        interrupts::without_interrupts(|| {
            self.current_task
                .borrow_mut()
                .fd_map
                .remove(&fd)
                .ok_or(io::Error::BadFileDescriptor)
        })
    }

    pub fn exit(&mut self) -> ! {
        interrupts::without_interrupts(|| {
            if self.current_task.borrow().status != TaskStatus::Idle {
                log::trace!("Finished task with id {:?}", self.current_task.borrow().id);
                self.current_task.borrow_mut().status = TaskStatus::Finished;
            } else {
                panic!("Cannot terminate idle task");
            }
        });

        self.reschedule();

        unreachable!("reschedule failed?")
    }

    pub fn schedule(&mut self) {
        // if we have finished tasks -> drop tasks -> deallocate stack (implicit)
        while let Some(task_id) = self.finished_tasks.pop_front() {
            if self.tasks.remove(&task_id).is_some() {
                // log::trace!("Dropping task with id {:?}", task_id);
                // ref count - 1
            } else {
                log::error!("Failed to drop task with id {:?} - not found", task_id);
            }
        }

        let (current_id, current_status, current_sp, current_priority) = {
            let mut b = self.current_task.borrow_mut();
            (b.id, b.status, &raw mut b.last_stack_ptr, b.priority)
        };

        let mut next_task: Option<RcTask>;
        if current_status == TaskStatus::Running {
            next_task = self.ready_queue.pop_with_priority(current_priority);
        } else {
            next_task = self.ready_queue.pop();
        }

        if next_task.is_none()
            && current_status != TaskStatus::Running
            && current_status != TaskStatus::Idle
        {
            // log::trace!("Switch to idle task");
            next_task = Some(Rc::clone(&self.idle_task));
        }

        if let Some(next_task) = next_task {
            let next_sp = {
                let mut b = next_task.borrow_mut();
                b.status = TaskStatus::Running;
                b.last_stack_ptr
            };

            if current_status == TaskStatus::Running {
                self.current_task.borrow_mut().status = TaskStatus::Ready;
                self.ready_queue.push(&self.current_task);
            } else if current_status == TaskStatus::Finished {
                // log::trace!(
                //     "Task with id {:?} has finished, adding to finished tasks",
                //     current_id
                // );

                self.finished_tasks.push_back(current_id);
            }

            self.current_task = next_task;
            unsafe { switch(current_sp, next_sp) };
        }
    }

    pub fn reschedule(&mut self) {
        interrupts::without_interrupts(|| self.schedule())
    }

    pub fn block_current_task(&self) -> RcTask {
        interrupts::without_interrupts(|| {
            if self.current_task.borrow().status == TaskStatus::Running {
                // log::trace!("Block task with id {:?}", self.current_task.borrow().id);

                self.current_task.borrow_mut().status = TaskStatus::Blocked;
                Rc::clone(&self.current_task)
            } else {
                panic!(
                    "Cannot block task with id {:?} - not running",
                    self.current_task.borrow().id
                );
            }
        })
    }

    pub fn wakeup_task(&mut self, task: &RcTask) {
        if task.borrow().status == TaskStatus::Blocked {
            // log::trace!("Waking up task id: {:?}", task.borrow().id);

            task.borrow_mut().status = TaskStatus::Ready;
            self.ready_queue.push(task);
        }
    }

    pub fn wakeup_task_by_id(&mut self, id: TaskId) {
        interrupts::without_interrupts(|| {
            if let Some(task) = self.tasks.get(&id)
                && task.borrow().status == TaskStatus::Blocked
            {
                // log::trace!("Waking up OS task id: {:?}", id);
                task.borrow_mut().status = TaskStatus::Ready;
                self.ready_queue.push(task);
            }
        });
    }

    pub fn get_current_task_id(&self) -> TaskId {
        interrupts::without_interrupts(|| self.current_task.borrow().id)
    }

    pub fn set_root_page_table(&self, addr: PhysicalAddress) {
        self.current_task.borrow_mut().root_page_table = addr;
    }

    pub fn get_root_page_table(&self) -> PhysicalAddress {
        self.current_task.borrow().root_page_table
    }
}

macro_rules! save_context {
    () => {
        "
        pushfq
        push rax
        push rcx
        push rdx
        push rbx
        sub  rsp, 8
        push rbp
        push rsi
        push rdi
        push r8
        push r9
        push r10
        push r11
        push r12
        push r13
        push r14
        push r15
        "
    };
}

macro_rules! restore_context {
    () => {
        "
        pop r15
        pop r14
        pop r13
        pop r12
        pop r11
        pop r10
        pop r9
        pop r8
        pop rdi
        pop rsi
        pop rbp
        add rsp, 8
        pop rbx
        pop rdx
        pop rcx
        pop rax
        popfq
        ret
        "
    };
}

#[unsafe(naked)]
pub(crate) unsafe extern "C" fn switch(_old_stack: *mut usize, _new_stack: usize) {
    // rdi = old_stack => the address to store the old rsp
    // rsi = new_stack => stack pointer of the new task

    core::arch::naked_asm! {
        save_context!(),
        "rdfsbase rax",
        "rdgsbase rdx",
        "push rax",
        "push rdx",
        // Store the old `rsp` behind `old_stack`
        "mov [rdi], rsp",
        // Set `rsp` to `new_stack`
        "mov rsp, rsi",
        // Set task switched flag
        "mov rax, cr0",
        "or rax, 8",
        "mov cr0, rax",
        // set stack pointer in TSS
        "call {set_stack}",
        "pop r15",
        "wrgsbase r15",
        "pop r15",
        "wrfsbase r15",
        restore_context!(),
        set_stack = sym set_current_kernel_stack,
    };
}

fn set_current_kernel_stack() {
    utils::write_cr3(super::get_root_page_table().0 as _);
    let current_stack = super::get_current_interrupt_stack();
    interrupts::set_kernel_stack(*current_stack);
}
