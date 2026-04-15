#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]
#![feature(linked_list_cursors)]

use crate::drivers::terminal::{Color, Reset};

extern crate alloc;

mod arch;
mod async_rt;
mod bus;
mod drivers;
mod elf;
mod errno;
mod fd;
mod fs;
mod io;
mod kernel;
mod limine_requests;
mod macros;
mod memory;
mod scheduler;
mod synch;
mod syscall;
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

    scheduler::reschedule();

    log::error!("Scheduler empty, main kernel thread entering idle loop");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);

    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
