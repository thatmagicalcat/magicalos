#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]

use crate::terminal::{Color, Reset};

extern crate alloc;

mod gdt;
mod hpet;
mod interrupts;
mod io;
mod ioapic;
mod kernel;
mod limine_requests;
mod macros;
mod memory;
mod scheduler;
mod task;
mod terminal;
mod thread;
mod utils;
mod volatile;

#[rustfmt::skip]
const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    kernel::init();

    println!("hello, world");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
