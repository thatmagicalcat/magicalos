use bitflags::bitflags;

use crate::println;

pub use idt::IDT;

#[macro_use]
mod macros;
mod handlers;
mod idt;
mod table;

pub use idt::*;

pub fn init() {
    IDT.load();
}

bitflags! {
    #[derive(Debug)]
    struct PageFaultErrorCode: u64 {
        const PROTECTION_VIOLATION = 1 << 0;
        const CAUSED_BY_WRITE = 1 << 1;
        const USER_MODE = 1 << 2;
        const MALFORMED_TABLE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
    }
}

pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) rflags,
            options(nomem, nostack, preserves_flags)
        );
    }

    rflags & (1 << 9) != 0
}
