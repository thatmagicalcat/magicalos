#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]
#![feature(linked_list_cursors)]

use core::time::Duration;

use crate::{
    scheduler::NORMAL_PRIORITY,
    syscall::Syscall,
    terminal::{Color, Reset},
};

extern crate alloc;

mod arch;
mod bus;
mod kernel;
mod limine_requests;
mod macros;
mod memory;
mod scheduler;
mod synch;
mod syscall;
mod async_rt;
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

    scheduler::spawn(create_user_cat, NORMAL_PRIORITY).unwrap();
    scheduler::spawn(f, NORMAL_PRIORITY).unwrap();

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

extern "C" fn create_user_cat() {
    utils::write_cr3(*memory::paging::user::create_user_page_table() as _);
    memory::paging::user::map_user_entry(user_cat);
    unsafe { arch::processor::jump_to_user_fn(user_cat) }
}

extern "C" fn user_cat() {
    let msg = *b"Hello, World\r\n";

    syscall!(Syscall::Write, msg.as_ptr(), msg.len());
    syscall!(Syscall::Exit);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);

    log::error!("KERNEL PANIC: {info}",);
    loop {}
}
