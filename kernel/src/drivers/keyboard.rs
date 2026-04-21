use core::{
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, stream::StreamExt, task::AtomicWaker};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};
use spin::Once;

use crate::{
    arch::{apic, interrupts, ioapic},
    scheduler,
    synch::Mutex,
};

static SCANCODE_QUEUE: Once<ArrayQueue<u8>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();
pub static KYEBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());

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
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
            && let Some(key) = keyboard.process_keyevent(key_event.clone())
        {
            let mut stdin = KYEBOARD_STATE.lock();
            match key {
                DecodedKey::Unicode(character) => match character {
                    // backspace
                    '\x08' if stdin.current_line.pop().is_some() => {
                        crate::drivers::terminal::backspace()
                    }

                    '\x08' => {}

                    '\n' => {
                        crate::println!();
                        stdin.current_line.push(b'\n');
                        let v = core::mem::take(&mut stdin.current_line);
                        stdin.ready_buffer.extend(v);
                        stdin.lines_ready += 1;

                        if let Some(task_id) = stdin.waiters.pop_front() {
                            scheduler::wakeup_task_by_id(task_id);
                        }
                    }

                    _ => {
                        crate::print!("{character}");
                        stdin
                            .current_line
                            .extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes());
                    }
                },

                DecodedKey::RawKey(_key) => {}
            }
        }
    }
}
