use core::{
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, stream::StreamExt, task::AtomicWaker};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};
use spin::Once;

static SCANCODE_QUEUE: Once<ArrayQueue<u8>> = Once::new();
static WAKER: AtomicWaker = AtomicWaker::new();

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

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE
            .get()
            .expect("ScancodeStream not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            log::trace!("Scancode: {scancode:#02x}");
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(ctx.waker());

        // double check
        match queue.pop() {
            Some(scancode) => {
                _ = WAKER.take();
                log::trace!("Scancode: {scancode:#02x}");
                Poll::Ready(Some(scancode))
            }

            None => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
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
            crate::print!("{key_event:?} -> ");
            match key {
                DecodedKey::Unicode(character) => crate::println!("{}", character),
                DecodedKey::RawKey(key) => crate::println!("{:?}", key),
            }
        }
    }
}
