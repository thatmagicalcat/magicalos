use core::task::Waker;

use alloc::{sync::Arc, task::Wake};
use crossbeam_queue::ArrayQueue;

use super::TaskId;

type TaskQueue = ArrayQueue<TaskId>;

pub fn create_waker(task_id: TaskId, ready_queue: Arc<TaskQueue>) -> Waker {
    Waker::from(Arc::new(TaskWaker {
        task_id,
        ready_queue,
    }))
}

struct TaskWaker {
    task_id: TaskId,
    ready_queue: Arc<TaskQueue>,
}

impl TaskWaker {
    fn wake_task(&self) {
        self.ready_queue
            .push(self.task_id)
            .expect("Task queue is full");
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
