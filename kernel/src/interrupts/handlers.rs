use core::arch::asm;

use crate::io::{apic, port::Port};

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

#[unsafe(naked)]
pub extern "C" fn reschedule_wrapper() -> ! {
    extern "C" fn reschedule_handler_raw(current_rsp: u64) -> u64 {
        crate::scheduler::schedule(current_rsp, true) // normal timer interrupt
    }

    core::arch::naked_asm! {
        // save current registers
        "push r15", "push r14", "push r13", "push r12",
        "push r11", "push r10", "push r9", "push r8",
        "push rbp", "push rdi", "push rsi", "push rdx",
        "push rcx", "push rbx", "push rax",

        "mov rdi, rsp", // first arg

        // 16 bytes alignment for the call
        "add rsp, -16",
        "call {handler}", // -> rax

        // switch to the new stack
        "mov rsp, rax",

        // load the registers from the new context
        "pop rax", "pop rbx", "pop rcx", "pop rdx",
        "pop rsi", "pop rdi", "pop rbp", "pop r8",
        "pop r9", "pop r10", "pop r11", "pop r12",
        "pop r13", "pop r14", "pop r15",

        "iretq",

        handler = sym reschedule_handler_raw
    }
}

#[unsafe(naked)]
pub extern "C" fn apic_timer_wrapper() -> ! {
    extern "C" fn apic_timer_handler_raw(current_rsp: u64) -> u64 {
        crate::io::apic::send_eoi();
        crate::scheduler::schedule(current_rsp, false) // normal timer interrupt
    }

    core::arch::naked_asm! {
        // save current registers
        "push r15", "push r14", "push r13", "push r12",
        "push r11", "push r10", "push r9", "push r8",
        "push rbp", "push rdi", "push rsi", "push rdx",
        "push rcx", "push rbx", "push rax",

        "mov rdi, rsp", // first arg

        // 16 bytes alignment for the call
        "add rsp, -16",
        "call {handler}", // -> rax

        // switch to the new stack
        "mov rsp, rax",

        // load the registers from the new context
        "pop rax", "pop rbx", "pop rcx", "pop rdx",
        "pop rsi", "pop rdi", "pop rbp", "pop r8",
        "pop r9", "pop r10", "pop r11", "pop r12",
        "pop r13", "pop r14", "pop r15",

        "iretq",

        handler = sym apic_timer_handler_raw
    }
}

pub extern "C" fn keyboard_handler(_stack_frame: &ExceptionStackFrame) {
    let scancode = unsafe { u8::read_from_port(0x60) };
    crate::task::keyboard::add_scancode(scancode);
    apic::send_eoi();
}
