use core::arch::naked_asm;

use super::{
    handlers::*,
    table::Idt,
};

const DIVIDE_BY_ZERO: u8 = 0;
const PAGE_FAULT: u8 = 14;
const BREAKPOINT: u8 = 3;

lazy_static::lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(DIVIDE_BY_ZERO, exception_handler!(divide_by_zero_handler));
        idt.set_handler(PAGE_FAULT, exception_handler_with_error_code!(page_fault_handler));
        idt.set_handler(BREAKPOINT, exception_handler!(breakpoint_handler));

        idt
    };
}
