use crate::vga_buffer::WRITER;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::macros::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
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
    use core::fmt::Write;
    crate::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}
