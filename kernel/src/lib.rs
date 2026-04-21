#![no_std]
#![cfg_attr(all(test, target_os = "none"), no_main)]
#![warn(clippy::missing_const_for_fn)]
#![feature(custom_test_frameworks)]
#![feature(linked_list_cursors)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

pub mod arch;
pub mod async_rt;
pub mod auxvec;
pub mod bus;
pub mod drivers;
pub mod elf;
pub mod errno;
pub mod fd;
pub mod fs;
pub mod io;
pub mod kernel;
pub mod limine_requests;
pub mod macros;
pub mod memory;
pub mod scheduler;
pub mod synch;
pub mod syscall;
pub mod testing;
pub mod utils;

#[rustfmt::skip]
pub(crate) const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info" )] { log::LevelFilter::Info  }
    #[cfg(log_level = "warn" )] { log::LevelFilter::Warn  }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

pub fn kentry() {
    kernel::init();

    let path = c"/home/thatmagicalcat/main.elf";
    scheduler::spawn(
        move || {
            scheduler::with_current_task(|task| {
                task.envp.push(c"FOO=BAR".into());
                task.argv.push(path.into());
            });

            elf::run(path.to_str().unwrap());
        },
        scheduler::NORMAL_PRIORITY,
    )
    .unwrap();

    scheduler::spawn(
        || {
            let mut async_rt = async_rt::Executor::new();
            async_rt.spawn(drivers::keyboard::handle_keypresses());
            async_rt.run();
        },
        scheduler::REALTIME_PRIORITY,
    )
    .unwrap();

    scheduler::reschedule();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[cfg(all(test, target_os = "none"))]
#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    test_main();
    testing::exit_qemu(testing::QemuExitCode::Success)
}

#[cfg(all(test, target_os = "none"))]
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    testing::test_panic_handler(info)
}

#[cfg(not(test))]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}");

    let has_terminal = drivers::terminal::TERMINAL.lock().is_some();
    if has_terminal {
        use drivers::terminal::{Color, Reset};
        println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);
    } else {
        dbg_println!("KERNEL PANIC: {info}");
    }

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
