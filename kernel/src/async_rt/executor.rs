use core::{
    any::type_name,
    task::{Context, Poll, Waker},
};

use alloc::{collections::BTreeMap, sync::Arc};
use crossbeam_queue::ArrayQueue;

use crate::scheduler;

use super::{Task, TaskId, waker::create_waker};

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    ready_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
    os_task_id: Option<scheduler::TaskId>,
}

impl Executor {
    pub fn new() -> Self {
        log::info!("Creating Task Executor");

        Self {
            tasks: BTreeMap::new(),
            ready_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
            os_task_id: None,
        }
    }

    pub fn spawn<T: Into<Task>>(&mut self, task: T) {
        let task = task.into();
        log::info!("Spawning task #{}, [{}]", task.id.0, type_name::<T>());

        let task_id = task.id;
        self.tasks.insert(task_id, task);
        self.ready_queue.push(task_id).expect("Task queue is full");
    }

    pub fn run(&mut self) -> ! {
        log::info!("Running Task Executor");
        self.os_task_id = Some(scheduler::get_current_task_id());

        loop {
            self.run_queued_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        if self.ready_queue.is_empty() {
            scheduler::block_current_task();
            scheduler::reschedule();
        }
    }

    fn run_queued_tasks(&mut self) {
        let Some(os_task_id) = self.os_task_id else {
            unreachable!("Executor is not running")
        };

        while let Some(task_id) = self.ready_queue.pop()
            && let Some(task) = self.tasks.get_mut(&task_id)
        {
            let waker: &mut Waker = self.waker_cache.entry(task_id).or_insert_with(|| {
                create_waker(task_id, Arc::clone(&self.ready_queue), os_task_id)
            });

            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Pending => {}
                Poll::Ready(()) => {
                    log::info!("Task #{} is finished!", task_id.0);
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
            }
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
