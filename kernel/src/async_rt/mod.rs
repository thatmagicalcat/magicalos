use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use alloc::boxed::Box;

pub use executor::Executor;

mod executor;
pub mod timer;
mod waker;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl core::fmt::Display for TaskId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Task {
    pub id: TaskId,
    pub future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
}

impl Task {
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    pub fn poll(&mut self, ctx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(ctx)
    }
}

impl<F> From<F> for Task
where
    F: Future<Output = ()> + Send + 'static,
{
    fn from(future: F) -> Self {
        Task::new(future)
    }
}

pub fn yield_now() -> impl Future<Output = ()> {
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldNow { yielded: false }
}
