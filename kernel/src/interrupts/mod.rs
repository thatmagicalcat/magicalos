use bitflags::bitflags;

use crate::println;

pub use idt::IDT;

mod handlers;
mod idt;
mod table;

pub use idt::*;

pub fn init() {
    log::info!("Initializing Interrupt Descriptor Table...");
    IDT.load();
    log::info!("IDT initialized.");
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

#[inline(always)]
pub fn disable_interrupts() {
    unsafe { core::arch::asm!("cli", options(nomem, nostack, preserves_flags)) };
}

#[inline(always)]
pub fn enable_interrupts() {
    unsafe { core::arch::asm!("sti", options(nomem, nostack, preserves_flags)) };
}

pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = interrupts_enabled();
    if was_enabled {
        disable_interrupts();
    }

    let r = f();

    if was_enabled {
        enable_interrupts();
    }

    r
}
