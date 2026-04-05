#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]

extern crate alloc;

mod gdt;
mod hpet;
mod interrupts;
mod io;
mod ioapic;
mod kernel;
mod macros;
mod memory;
mod scheduler;
mod task;
mod thread;
mod utils;
mod volatile;
mod limine_requests;

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

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
