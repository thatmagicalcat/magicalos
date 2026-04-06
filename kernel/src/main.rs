#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]

use crate::{scheduler::{HIGH_PRIORITY, REALTIME_PRIORITY}, terminal::{Color, Reset, TERMINAL}};

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

    extern "C" fn t1() {
        for i in 0..10 {
            println!("Task 1: iteration {i}");
            scheduler::reschedule();
        }
    }

    extern "C" fn t2() {
        for i in 0..10 {
            println!("Task 2: iteration {i}");
            scheduler::reschedule();
        }
    }

    scheduler::spawn(t1, HIGH_PRIORITY).unwrap();
    scheduler::spawn(t2, REALTIME_PRIORITY).unwrap();

    // let the scheduler take over
    scheduler::reschedule();

    log::error!("Back to kernel?!?!?");

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
