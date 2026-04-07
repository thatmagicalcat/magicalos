/// A fair ticket locl
use core::sync::atomic::{AtomicUsize, Ordering};

use lock_api::{GuardSend, RawMutex, RawMutexFair};

pub struct RawSpinlock {
    next_ticket: AtomicUsize,
    now_serving: AtomicUsize,
}

unsafe impl RawMutex for RawSpinlock {
    const INIT: Self = Self {
        next_ticket: AtomicUsize::new(0),
        now_serving: AtomicUsize::new(0),
    };

    type GuardMarker = GuardSend;

    #[inline]
    fn lock(&self) {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);
        while self.now_serving.load(Ordering::Acquire) != ticket {
            core::hint::spin_loop();
        }
    }

    #[inline]
    fn try_lock(&self) -> bool {
        self.next_ticket
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ticket| {
                if self.now_serving.load(Ordering::Acquire) == ticket {
                    Some(ticket + 1)
                } else {
                    None
                }
            })
            .is_ok()
    }

    #[inline]
    unsafe fn unlock(&self) {
        self.now_serving.fetch_add(1, Ordering::Release);
    }
}

unsafe impl RawMutexFair for RawSpinlock {
    #[inline]
    unsafe fn unlock_fair(&self) {
        unsafe { self.unlock() };
    }

    #[inline]
    unsafe fn bump(&self) {
        let ticket = self.next_ticket.load(Ordering::Relaxed);
        let serving = self.now_serving.load(Ordering::Relaxed);

        if serving + 1 != ticket {
            unsafe {
                self.unlock_fair();
                self.lock();
            }
        }
    }
}

pub type Spinlock<T> = lock_api::Mutex<RawSpinlock, T>;
pub type SpinlockGuard<'a, T> = lock_api::MutexGuard<'a, RawSpinlock, T>;
