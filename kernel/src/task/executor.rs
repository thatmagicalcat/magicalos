use core::task::{Context, Poll, Waker};

use alloc::{collections::BTreeMap, sync::Arc};
use crossbeam_queue::ArrayQueue;

use crate::interrupts;

use super::{Task, TaskId, waker::create_waker};

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    ready_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            ready_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: impl Into<Task>) {
        let task = task.into();
        let task_id = task.id;
        self.tasks.insert(task_id, task);
        self.ready_queue.push(task_id).expect("Task queue is full");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_queued_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        interrupts::disable_interrupts();

        if self.ready_queue.is_empty() {
            unsafe { core::arch::asm!("sti; hlt", options(nomem, nostack)) };
        } else {
            interrupts::enable_interrupts();
        }
    }

    fn run_queued_tasks(&mut self) {
        while let Some(task_id) = self.ready_queue.pop()
            && let Some(task) = self.tasks.get_mut(&task_id)
        {
            let waker: &mut Waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| create_waker(task_id, Arc::clone(&self.ready_queue)));

            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Pending => {}
                Poll::Ready(()) => {
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
            }
        }
    }
}
