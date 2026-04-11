use core::fmt::Write;
use crate::arch::interrupts;

use crate::{
    io::{qemu_debug::QEMU_DEBUGCON},
    terminal::TERMINAL,
};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::macros::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => ($crate::print!("{}\r\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! dbg_print {
    ($($arg:tt)*) => ($crate::macros::_dbg_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! dbg_println {
    () => ($crate::dbg_print!("\n"));
    ($($arg:tt)*) => ($crate::dbg_print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! dbg {
    [ $e:expr ] => {{
        let result = $e;
        log::debug!("{} = {result:?}", stringify!($e));
        result
    }};

    [ $($e:expr),* $(,)? ] => { $($crate::dbg!($e);)* }
}

#[macro_export]
macro_rules! breakpoint {
    [] => {
        unsafe {
            core::arch::asm! {
                "int3",
                options(nomem, nostack)
            }
        };
    };
}

#[macro_export]
macro_rules! flush_tlb {
    // Flush all
    [] => {
        let value: u64;
        unsafe {
            asm! {
                "mov {}, cr3",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            }

            asm! {
                "mov cr3, {}",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            }
        };
    };

    // Flush a single page
    [ $page:expr ] => {
        unsafe {
            asm!("invlpg [{}]", in(reg) $page, options(nostack, preserves_flags))
        };
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        TERMINAL.lock().as_mut().unwrap().write_fmt(args).unwrap();
    });
}

#[doc(hidden)]
pub fn _dbg_print(args: core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        QEMU_DEBUGCON.lock().write_fmt(args).unwrap();
    });
}
