use core::task::Waker;

use alloc::{sync::Arc, task::Wake};
use crossbeam_queue::ArrayQueue;

use crate::scheduler;

use super::TaskId;

type TaskQueue = ArrayQueue<TaskId>;

pub fn create_waker(
    task_id: TaskId,
    ready_queue: Arc<TaskQueue>,
    os_task_id: scheduler::TaskId,
) -> Waker {
    Waker::from(Arc::new(TaskWaker {
        async_task_id: task_id,
        ready_queue,
        os_task_id,
    }))
}

struct TaskWaker {
    async_task_id: TaskId,
    ready_queue: Arc<TaskQueue>,
    os_task_id: scheduler::TaskId,
}

impl TaskWaker {
    fn wake_task(&self) {
        if self.ready_queue.push(self.async_task_id).is_ok() {
            scheduler::wakeup_task_by_id(self.os_task_id);
        } else {
            log::error!("Task queue is full")
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
