use bitflags::bitflags;

use crate::println;

pub use idt::IDT;

#[macro_use]
mod macros;
mod handlers;
mod idt;
mod table;

pub use idt::{Tss, TSS};

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
