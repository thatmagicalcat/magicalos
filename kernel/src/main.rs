#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]

use core::alloc::Layout;

use alloc::alloc::alloc;

use crate::{
    scheduler::{NORMAL_PRIORITY, REALTIME_PRIORITY},
    terminal::{Color, Reset},
};

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
mod synch;
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

    // let fn_addr = kernel::get_user_fn_address(user_task).0;
    // let fn_ptr: extern "C" fn() =
    //     unsafe { core::mem::transmute::<usize, extern "C" fn()>(fn_addr as usize) };
    //
    // log::info!("calling");
    // fn_ptr();
    //
    scheduler::spawn(create_user_task, NORMAL_PRIORITY).unwrap();

    // let the scheduler take over
    scheduler::reschedule();

    log::error!("Scheduler empty, main kernel thread entering idle loop");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

// allocates a stack and jumps to userland
#[allow(static_mut_refs)] // shut the fuck up
extern "C" fn create_user_task() {
    const STACK_SIZE: usize = 4096;
    static mut STACK: Option<usize> = None;

    if unsafe { STACK.is_none() } {
        log::debug!("Allocating stack for user task");

        let layout = Layout::from_size_align(   STACK_SIZE, 16).unwrap();
        let stack_mem = unsafe { alloc(layout) };
        unsafe { STACK = Some(stack_mem as usize + STACK_SIZE) };
    }

    let stack_ptr = unsafe { STACK.unwrap() };
    let entry_point = kernel::get_user_fn_address(user_task);

    unsafe { kernel::jump_to_user_fn(*entry_point, stack_ptr as _) };
}

#[unsafe(link_section = ".userland")]
extern "C" fn user_task() {
    // will cause a #PF
    let x = 10;
    core::hint::black_box(&x);
    loop {
        core::hint::spin_loop();
    }
}

#[used]
static F: extern "C" fn() = user_task;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);

    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
