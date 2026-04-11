use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

use alloc::{collections::BTreeMap, vec::Vec};
use futures_util::task::AtomicWaker;
use spin::Mutex;

use crate::{arch::hpet::HPET, io::apic, utils};

type Nanoseconds = u64;

pub static WAKER: AtomicWaker = AtomicWaker::new();
pub static TIMERS: Mutex<BTreeMap<Nanoseconds, Vec<Waker>>> = Mutex::new(BTreeMap::new());

fn get_min_timestamp() -> Option<u64> {
    TIMERS.lock().first_key_value().map(|(tick, _)| *tick)
}

fn stop_timer() {
    apic::set_timer(
        apic::DivideConfig::DivideBy1,
        0,
        apic::LvtTimerMode::ONESHOT,
    );
}

struct Timer {
    target: Nanoseconds,
    timer_started: bool,
}

impl Timer {
    const fn new(target: Nanoseconds) -> Self {
        Self {
            target,
            timer_started: false,
        }
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let currnet_timestamp_nanos = HPET.get().unwrap().uptime_nanos();

        // condition 1: maybe waken up by the interrupt

        // we want to hook the waker if target is zero
        // so that this future can be polled again from the sleep future
        if self.target != 0 && currnet_timestamp_nanos >= self.target {
            return Poll::Ready(());
        }

        WAKER.register(cx.waker());

        // condition 2: polled for the first time
        if !self.timer_started {
            self.as_mut().timer_started = true;

            // the timer is now registered, now we can return pending
            if self.target == 0 {
                log::trace!("No timers registered, waiting for waker");
                return Poll::Pending;
            }

            let utils::LApicConfig {
                initial_count,
                divide_config,
            } = utils::duration_to_timer_config(
                self.target - currnet_timestamp_nanos,
                apic::get_timer_frequency(),
            )
            .expect("Duration too long to convert to ticks");

            apic::set_timer(divide_config, initial_count, apic::LvtTimerMode::ONESHOT);

            if initial_count == 0 {
                WAKER.wake();
            }

            return Poll::Pending;
        }

        // condition 3: waken up my sleep future because it has minimum time
        stop_timer();
        Poll::Ready(())
    }
}

struct Sleep {
    target: Nanoseconds,
    registered: bool,
}

impl Sleep {
    const fn new(target: Nanoseconds) -> Self {
        Self {
            target,
            registered: false,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current_timestamp = HPET.get().unwrap().uptime_nanos();

        if current_timestamp >= self.target {
            return Poll::Ready(());
        }

        if !self.registered {
            self.as_mut().registered = true;

            let is_earliest = TIMERS
                .lock()
                .first_key_value()
                .map(|(earliest, _)| *earliest > self.target)
                .unwrap_or(true); // only timer, must be the earliest

            TIMERS
                .lock()
                .entry(self.target)
                .or_default()
                .push(cx.waker().clone());

            if is_earliest {
                // wake up the previous so it that timer_dispatch function can setup
                // a new timer for this new minimum sleep
                WAKER.wake();
            }
        }

        Poll::Pending
    }
}

pub async fn timer_dispatch() {
    loop {
        loop {
            let current_timestamp = HPET.get().unwrap().uptime_nanos();
            let mut timers = TIMERS.lock();

            if let Some((&next_timestamp, _)) = timers.first_key_value()
                && current_timestamp >= next_timestamp
            {
                // the time is up!
                let (_, wakers) = timers.pop_first().unwrap(); // safe

                // free the lock so that next sleep timers don't have to wait until we are
                // waking up previous sleeps
                drop(timers);

                for waker in wakers {
                    waker.wake();
                }

                // there could be more timers, which are expired because we spent significant
                // amount of time waking up
                continue;
            }

            // TIMERS list is empty, no need to check
            break;
        }

        let next_timestamp = get_min_timestamp().unwrap_or(0);
        Timer::new(next_timestamp).await;
    }
}

pub async fn sleep(duration: Duration) {
    Sleep::new(HPET.get().unwrap().uptime_nanos() + duration.as_nanos() as u64).await;
}
