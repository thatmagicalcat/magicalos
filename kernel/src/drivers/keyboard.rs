use core::{
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, stream::StreamExt, task::AtomicWaker};
use pc_keyboard::{DecodedKey, HandleControl, KeyState, Keyboard, ScancodeSet1, layouts};
use spin::Once;

use crate::{
    arch::{apic, interrupts, ioapic},
    io::{self, IoInterface},
    scheduler,
    synch::Mutex,
};

static SCANCODE_QUEUE: Once<ArrayQueue<u8>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();

pub static KYEBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
pub static KEY_EVENT_QUEUE: Mutex<KeyboardEventQueue> = Mutex::new(KeyboardEventQueue::new());

#[derive(Debug)]
pub struct KeyboardEventDevice;

impl IoInterface for KeyboardEventDevice {
    // Non-blocking
    fn read(&self, buf: &mut [u8]) -> crate::io::Result<usize> {
        const EVENT_SIZE: usize = core::mem::size_of::<RawKeyEvent>();
        if buf.len() < EVENT_SIZE {
            return Err(io::Error::InvalidValue);
        }
        let mut queue = KEY_EVENT_QUEUE.lock();
        
        //  If there are no key events, return 0 bytes read immediately.
        if queue.events.is_empty() {
            return Ok(0);
        }

        // how many events can we push?
        let buffer_event_capacity = buf.len() / EVENT_SIZE;
        let events_to_read = queue.events.len().min(buffer_event_capacity);
        let mut bytes_written = 0;

        for _ in 0..events_to_read {
            // SAFETY: events_to_read <= events.len()
            let e = queue.events.pop_front().unwrap();
            let event_slice =
                unsafe { core::slice::from_raw_parts(&raw const e as *const u8, EVENT_SIZE) };

            buf[bytes_written..bytes_written + EVENT_SIZE].copy_from_slice(event_slice);
            bytes_written += EVENT_SIZE;
        }

        Ok(bytes_written)
    }

    // Blocking
    // fn read(&self, buf: &mut [u8]) -> crate::io::Result<usize> {
    //     const EVENT_SIZE: usize = core::mem::size_of::<RawKeyEvent>();
    //
    //     if buf.len() < EVENT_SIZE {
    //         return Err(io::Error::InvalidValue);
    //     }
    //
    //     loop {
    //         let mut queue = KEY_EVENT_QUEUE.lock();
    //
    //         if queue.events.is_empty() {
    //             queue.add_waiter(scheduler::get_current_task_id());
    //             drop(queue);
    //
    //             scheduler::block_current_task();
    //             scheduler::reschedule();
    //
    //             continue;
    //         }
    //
    //         // how many events can we push?
    //         let buffer_event_capacity = buf.len() / EVENT_SIZE;
    //         let events_to_read = queue.events.len().min(buffer_event_capacity);
    //         let mut bytes_written = 0;
    //
    //         for _ in 0..events_to_read {
    //             // SAFETY: events_to_read <= events.len()
    //             let e = queue.events.pop_front().unwrap();
    //             let event_slice =
    //                 unsafe { core::slice::from_raw_parts(&raw const e as *const u8, EVENT_SIZE) };
    //
    //             buf[bytes_written..bytes_written + EVENT_SIZE].copy_from_slice(event_slice);
    //             bytes_written += EVENT_SIZE;
    //         }
    //
    //         return Ok(bytes_written);
    //     }
    // }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawKeyEvent {
    /// Timestamp in nanoseconds since boot
    pub timestamp_nanos: u64,
    /// The key code
    pub code: u8,
    /// 1 - down, 0 - up
    pub state: u8,
}

pub struct KeyboardEventQueue {
    pub events: VecDeque<RawKeyEvent>,
    pub waiters: VecDeque<scheduler::TaskId>,
}

impl KeyboardEventQueue {
    fn add_waiter(&mut self, task_id: scheduler::TaskId) {
        if !self.waiters.contains(&task_id) {
            self.waiters.push_back(task_id);
        }
    }
}

impl KeyboardEventQueue {
    const fn new() -> Self {
        Self {
            events: VecDeque::new(),
            waiters: VecDeque::new(),
        }
    }
}

pub struct KeyboardState {
    pub lines_ready: usize,
    /// Lines ready to be read
    pub ready_buffer: VecDeque<u8>,
    /// Current line which is being read
    current_line: Vec<u8>,

    waiters: VecDeque<scheduler::TaskId>,
}

impl KeyboardState {
    pub const fn new() -> Self {
        Self {
            lines_ready: 0,
            ready_buffer: VecDeque::new(),
            current_line: Vec::new(),
            waiters: VecDeque::new(),
        }
    }

    pub fn add_waiter(&mut self, task_id: scheduler::TaskId) {
        if !self.waiters.contains(&task_id) {
            self.waiters.push_back(task_id);
        }
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init() {
    // enable keyboard interrupt
    // TODO: find the correct GSI for the keyboard instead of hardcoding it to 1
    ioapic::enable_irq(
        1,
        interrupts::InterruptEntryType::Keyboard as _,
        apic::get_id(),
    );

    let mut lock = KEY_EVENT_QUEUE.lock();
    lock.events.reserve(256);
    lock.waiters.reserve(10);

    let mut lock = KYEBOARD_STATE.lock();
    lock.ready_buffer.reserve(256);
    lock.current_line.reserve(26);
    lock.waiters.reserve(10);
}

pub fn add_scancode(scancode: u8) {
    if let Some(queue) = SCANCODE_QUEUE.get() {
        if queue.push(scancode).is_err() {
            log::warn!("Scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }

        return;
    }

    log::warn!("Scancode queue uninitialized; dropping keyboard input");
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.call_once(|| ArrayQueue::new(100));
        ScancodeStream { _private: () }
    }
}

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .get()
            .expect("ScancodeStream not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(ctx.waker());

        // double check
        match queue.pop() {
            Some(scancode) => {
                _ = WAKER.take();
                Poll::Ready(Some(scancode))
            }

            None => Poll::Pending,
        }
    }
}

pub async fn handle_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            let timestamp_nanos = crate::arch::hpet::HPET
                .get()
                .map(|hpet| hpet.uptime_nanos())
                .unwrap_or_else(|| {
                    log::error!("HPET is not initialized! Setting timestamp to 0");
                    0 // fallback to 0
                });

            let raw_event = RawKeyEvent {
                timestamp_nanos,
                code: key_event.code as _,
                state: if key_event.state == pc_keyboard::KeyState::Down {
                    1
                } else {
                    0
                },
            };

            let mut queue = KEY_EVENT_QUEUE.lock();

            if queue.events.len() < 256 {
                queue.events.push_back(raw_event);
            } else {
                log::warn!("Keyboard event queue full! Dropping event.");
            }

            // wake up ONE waiting process
            if let Some(task_id) = queue.waiters.pop_front() {
                scheduler::wakeup_task_by_id(task_id);
            }

            drop(queue);

            let Some(key) = keyboard.process_keyevent(key_event.clone()) else {
                continue;
            };

            let mut ks = KYEBOARD_STATE.lock();
            match key {
                DecodedKey::Unicode(character) => match character {
                    // backspace
                    '\x08' if ks.current_line.pop().is_some() => {
                        crate::drivers::terminal::backspace()
                    }

                    '\x08' => {}

                    '\n' => {
                        crate::println!();
                        ks.current_line.push(b'\n');
                        let v = core::mem::take(&mut ks.current_line);
                        ks.ready_buffer.extend(v);
                        ks.lines_ready += 1;

                        if let Some(task_id) = ks.waiters.pop_front() {
                            scheduler::wakeup_task_by_id(task_id);
                        }
                    }

                    _ => {
                        crate::print!("{character}");
                        ks.current_line
                            .extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
                    }
                },

                DecodedKey::RawKey(_key) => {}
            }
        }
    }
}
