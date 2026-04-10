use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use crate::scheduler::{self, PriorityTaskQueue};

use super::Spinlock;

pub struct Mutex<T: ?Sized> {
    locked: Spinlock<bool>,
    queue: Spinlock<PriorityTaskQueue>,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    locked: &'a Spinlock<bool>,
    queue: &'a Spinlock<PriorityTaskQueue>,
    data: &'a mut T,
}

/// SAFETY: trust me bro
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: Spinlock::new(false),
            queue: Spinlock::new(PriorityTaskQueue::new()),
            data: UnsafeCell::new(data),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    fn obtain_lock(&self) {
        loop {
            let mut locked = self.locked.lock();

            if !*locked {
                *locked = true;
                return;
            } else {
                self.queue.lock().push(&scheduler::block_current_task());
                drop(locked); // release lock
                scheduler::reschedule(); // switch to next task
            }
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.obtain_lock();
        MutexGuard {
            locked: &self.locked,
            queue: &self.queue,
            data: unsafe { &mut *self.data.get() },
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        let mut locked = self.locked.lock();
        *locked = false;

        // try to wakeup next task
        if let Some(task) = self.queue.lock().pop() {
            scheduler::wakeup_task(&task);
        }
    }
}
