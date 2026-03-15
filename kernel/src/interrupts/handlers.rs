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

pub extern "C" fn breakpoint_handler(stack_frame: &ExceptionStackFrame) {
    println!(
        "\nEXCEPTION: BREAKPOINT at {:#X}\n{:#?}",
        stack_frame.instr_ptr, stack_frame
    );
}

pub extern "C" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) {
    println!("\nEXCEPTION: DIVIDE BY ZERO\n{stack_frame:#?}");
}

pub extern "C" fn page_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) {
    unsafe {
        println!(
            "\nEXCEPTION: PAGE FAULT while accessing {:#x}\nError code: {:?}\n{:#?}",
            x86_64::registers::control::Cr2::read().unwrap_unchecked(),
            PageFaultErrorCode::from_bits(error_code).unwrap_unchecked(),
            stack_frame
        );
    }
}
