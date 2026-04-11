#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]
#![feature(linked_list_cursors)]

use core::time::Duration;

use crate::{
    arch::processor, drivers::terminal::{Color, Reset}, scheduler::NORMAL_PRIORITY, syscall::Syscall
};

extern crate alloc;

mod arch;
mod drivers;
mod bus;
mod kernel;
mod limine_requests;
mod macros;
mod memory;
mod scheduler;
mod synch;
mod syscall;
mod async_rt;
mod utils;
mod volatile;
mod io;
mod errno;
mod fd;

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

    scheduler::spawn(create_user_process, NORMAL_PRIORITY).unwrap();
    scheduler::spawn(kernel_process, NORMAL_PRIORITY).unwrap();

    scheduler::reschedule();

    log::error!("Scheduler empty, main kernel thread entering idle loop");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "C" fn f() {
    for i in 0..10 {
        println!("{i}");
        arch::hpet::HPET
            .get()
            .unwrap()
            .busy_wait(Duration::from_millis(50));
    }
}

/// A function to create a new userspace processes
extern "C" fn create_user_process() {
    utils::write_cr3(*memory::paging::user::create_page_table() as _);
    memory::paging::user::map_user_entry(user_process);

    unsafe { processor::jump_to_user_fn(user_process) }
}

extern "C" fn user_process() {
    // NOTE: println uses kernel's terminal driver which is located in RING 0
    // memory, using it will cause a protection violation.

    let msg = *b"Hello from a userspace process!\r\n";

    syscall!(Syscall::Write, fd::STDOUT_FILENO, msg.as_ptr(), msg.len());
    syscall!(Syscall::Exit);
}

extern "C" fn kernel_process() {
    // we can use println here, cuz this will run in RING 0
    println!("Hello from a kernel processes!");
}


#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);

    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
