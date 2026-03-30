// TODO: Use TSC instead of HPET for better performance

use core::time::Duration;

use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    vec::Vec,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    hpet::HPET,
    interrupts,
    thread::{Thread, ThreadState},
};

lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

pub fn init() {
    log::info!("Initializing an idle thread");
    interrupts::without_interrupts(|| {
        SCHEDULER.lock().init_idle();
    });
}

#[allow(clippy::vec_box)]
pub struct Scheduler {
    ready_queue: VecDeque<Box<Thread>>,

    /// Target HPET timestamp (in nanoseconds)
    sleep_queue: BTreeMap<u64, Vec<Box<Thread>>>,

    /// first is the timestamp at which the thread started running
    /// it is used to calculate the time slice of the thread, which is used for preemptive
    /// scheduling
    current_thread: Option<(u64, Box<Thread>)>,
    idle_thread: Option<Box<Thread>>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            sleep_queue: BTreeMap::new(),
            current_thread: None,
            idle_thread: None,
        }
    }

    fn init_idle(&mut self) {
        if self.idle_thread.is_some() {
            log::warn!("An idle thread is already initialized");
            return;
        }

        fn idle_fn() {
            loop {
                unsafe { core::arch::asm!("sti; hlt") };
            }
        }

        let mut idle_thread = Thread::new(idle_fn);
        idle_thread.state = ThreadState::Idle;
        self.idle_thread = Some(idle_thread);
    }
}

pub fn spawn(thread: Box<Thread>) {
    let mut sched = interrupts::without_interrupts(|| SCHEDULER.lock());
    log::info!("Spawning thread ID: {}", thread.id);
    sched.ready_queue.push_back(thread);
}

pub fn schedule(current_rsp: u64, resched: bool) -> u64 {
    let mut sched = interrupts::without_interrupts(|| SCHEDULER.lock());

    // move expired sleeping threads to the ready queue
    let currnet_timestamp_nanos = HPET.get().unwrap().uptime_nanos();
    while let Some((&target_timestamp_nanos, _)) = sched.sleep_queue.first_key_value()
        && currnet_timestamp_nanos >= target_timestamp_nanos
    {
        let (_, threads) = sched.sleep_queue.pop_first().unwrap(); // safe

        for mut thread in threads {
            log::trace!("Waking up thread ID: {}", thread.id);
            thread.state = ThreadState::Ready;
            sched.ready_queue.push_back(thread);
        }
    }

    const TIME_SLICE_NANOS: u64 = 10_000_000; // 10ms

    if let Some((timestamp, _)) = sched.current_thread {
        if !resched && currnet_timestamp_nanos - timestamp < TIME_SLICE_NANOS {
            // the current thread is still within its time slice, return the current rsp
            return current_rsp;
        }

        let (_, mut current_thread) = sched.current_thread.take().unwrap(); // safe
        current_thread.rsp = current_rsp;

        match current_thread.state {
            ThreadState::Running => {
                current_thread.state = ThreadState::Ready;
                sched.ready_queue.push_back(current_thread);
            }

            ThreadState::Sleeping(wake_time) => {
                sched
                    .sleep_queue
                    .entry(wake_time)
                    .or_default()
                    .push(current_thread);
            }

            ThreadState::Idle => {
                sched.idle_thread = Some(current_thread);
            }

            _ => {}
        }
    }

    // pick the next thread to run
    if let Some(mut next_thread) = sched.ready_queue.pop_front() {
        let next_rsp = next_thread.rsp;

        next_thread.state = ThreadState::Running;
        sched.current_thread = Some((currnet_timestamp_nanos, next_thread));
        next_rsp
    } else if let Some(idle_thread) = sched.idle_thread.take() {
        log::debug!("Switching to idle thread");

        let next_rsp = idle_thread.rsp;
        sched.current_thread = Some((currnet_timestamp_nanos, idle_thread));
        next_rsp
    } else {
        // no thread to run, return the current rsp
        log::warn!("No thread to schedule, returning current rsp");
        current_rsp
    }
}

pub fn yield_cpu() {
    log::trace!("Yielding current thread");

    assert_eq!(
        interrupts::InterruptEntryType::Reschedule as u8,
        0x22,
        "Reschedule interrupt entry index must be 0x22"
    );

    unsafe { core::arch::asm!("int 0x22") };
}

pub fn sleep(duration: Duration) {
    let scheduled = interrupts::without_interrupts(|| {
        let duration_nanos = duration.as_nanos() as u64;
        let current_time = HPET.get().unwrap().uptime_nanos();
        let wake_time = current_time + duration_nanos;

        let mut sched = SCHEDULER.lock();

        if let Some((_, current)) = &mut sched.current_thread {
            current.state = ThreadState::Sleeping(wake_time);
            true
        } else {
            false
        }
    });

    //  we're in early boot stage, so just busy-wait
    if !scheduled {
        HPET.get().unwrap().busy_wait(duration);
        return;
    }

    // the current thread is now sleeping
    // switch to a different thread
    yield_cpu();
}
