use core::arch::asm;

use crate::{apic, port::Port};

use super::*;

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    instr_ptr: usize,
    code_segment: usize,
    cpu_flags: usize,
    stack_ptr: usize,
    stack_segment: usize,
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: &ExceptionStackFrame) {
    println!(
        "\nEXCEPTION: BREAKPOINT at {:#X}\n{:#?}",
        stack_frame.instr_ptr, stack_frame
    );
}

pub extern "x86-interrupt" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) {
    panic!("\nEXCEPTION: DIVIDE BY ZERO\n{stack_frame:#?}");
}

pub extern "x86-interrupt" fn page_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) {
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

pub extern "x86-interrupt" fn double_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) -> ! {
    panic!(
        "\nEXCEPTION: DOUBLE FAULT\nError code: {error_code}\n{:#?}",
        stack_frame
    );
}

pub extern "x86-interrupt" fn spurious_interrupt_handler(stack_frame: &ExceptionStackFrame) {
    println!(
        "\nEXCEPTION: SPURIOUS INTERRUPT at {:#X}\n{:#?}",
        stack_frame.instr_ptr, stack_frame
    );
}

pub extern "x86-interrupt" fn apic_timer_handler(_stack_frame: &ExceptionStackFrame) {
    crate::task::timer::WAKER.wake();
    apic::send_eoi();
}

pub extern "x86-interrupt" fn keyboard_handler(_stack_frame: &ExceptionStackFrame) {
    let scancode = unsafe { u8::read_from_port(0x60) };
    crate::task::keyboard::add_scancode(scancode);
    apic::send_eoi();
}
