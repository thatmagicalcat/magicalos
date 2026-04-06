use core::arch::asm;

use crate::{io::{apic, port::Port}, scheduler};

use super::*;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    rip: usize,
    cs: usize,
    rflags: usize,
    rsp: usize,
    ss: usize,
}

pub extern "C" fn breakpoint_handler(stack_frame: &ExceptionStackFrame) {
    log::warn!(
        "\nEXCEPTION: BREAKPOINT at {:#X}\n{:#?}",
        stack_frame.rip,
        stack_frame
    );
}

pub extern "C" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) {
    panic!("\nEXCEPTION: DIVIDE BY ZERO\n{stack_frame:#?}");
}

pub extern "C" fn page_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) {
    let value: u64;

    unsafe {
        asm! {
            "mov {}, cr2",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        };

        panic!(
            "\nEXCEPTION: PAGE FAULT while accessing {:#x}\nError code: {:?}\n{:#?}",
            value,
            PageFaultErrorCode::from_bits(error_code).unwrap_unchecked(),
            stack_frame
        );
    }
}

pub extern "C" fn double_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) -> ! {
    panic!(
        "\nEXCEPTION: DOUBLE FAULT\nError code: {error_code}\n{:#?}",
        stack_frame
    );
}

pub extern "C" fn general_protection_fault_handler(
    stack_frame: &ExceptionStackFrame,
    error_code: u64,
) {
    let is_external = error_code & 1 != 0;
    let table = (error_code >> 1) & 0b11;
    let index = (error_code >> 3) & 0x1FFF;

    panic!(
        "\nEXCEPTION: GENERAL PROTECTION FAULT\nError code: {error_code} (external: {is_external}, table: {table}, index: {index})\n{:#?}",
        stack_frame
    );
}

pub extern "C" fn spurious_interrupt_handler(stack_frame: &ExceptionStackFrame) {
    log::warn!(
        "\nEXCEPTION: SPURIOUS INTERRUPT at {:#X}\n{:#?}",
        stack_frame.rip,
        stack_frame
    );
}

pub extern "C" fn acpi_timer_interrupt() {
    apic::send_eoi();
    scheduler::schedule();
}

pub extern "C" fn keyboard_handler(_stack_frame: &ExceptionStackFrame) {
    let scancode = unsafe { u8::read_from_port(0x60) };
    crate::task::keyboard::add_scancode(scancode);
    apic::send_eoi();
}
