use core::arch::asm;

use crate::{
    arch::apic,
    bus::port::Port,
    kernel::{USER_ENTRY, USER_STACK_BOTTOM, USER_STACK_TOP},
    limine_requests,
    memory::{
        self,
        paging::{PageTable, PageTableEntryFlags, VirtualAddress},
    },
    scheduler, utils,
};

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

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrameWithError {
    error_code: usize,
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

pub extern "C" fn stack_segment_fault(stack_frame: &ExceptionStackFrameWithError) {
    let error_code = stack_frame.error_code;
    log::warn!(
        "\nEXCEPTION: STACK-SEGMENT FAULT at {:#X}\nError code: {error_code}\n{:#?}",
        stack_frame.rip,
        stack_frame
    );
}

pub extern "C" fn invalid_opcode(stack_frame: &ExceptionStackFrame) {
    panic!(
        "\nEXCEPTION: INVALID OPCODE at {:#X}\n{:#?}",
        stack_frame.rip, stack_frame
    );
}

pub extern "C" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) {
    panic!("\nEXCEPTION: DIVIDE BY ZERO\n{stack_frame:#?}");
}

pub extern "C" fn page_fault_handler(stack_frame: &ExceptionStackFrameWithError) {
    let mut virtual_addr: usize;
    let error_code = stack_frame.error_code;

    unsafe {
        asm! {
            "mov {}, cr2",
            out(reg) virtual_addr,
            options(nomem, nostack, preserves_flags)
        };
    }

    const RED_ZONE: u64 = 128; // bytes

    let user_rsp = stack_frame.rsp as u64;

    // check if the virtual_addr falls in the 8 MiB range
    let is_in_stack_range =
        (USER_STACK_BOTTOM.0..USER_STACK_TOP.0).contains(&(virtual_addr as _));

    // we reserve memory by subtracting rsp by a certain amount, so if the rsp was subtracted
    // before and then we're accessing that memory, it is only valid if the virtual_addr is >= rsp
    // but if this condition fails, that means the user is trying to access memory that is not even
    // reserved!
    // TODO: kill that process!

    let is_valid_stack_growth = virtual_addr as u64 >= (user_rsp.saturating_sub(RED_ZONE));

    if is_in_stack_range && is_valid_stack_growth {
        // Safe to demand page!

        virtual_addr = utils::align_down(virtual_addr, memory::PAGE_SIZE) as _;
        let physical_frame = memory::allocate_frame().expect("oom");
        let hhdm_offset = unsafe { (*limine_requests::HHDM_REQUEST.response).offset } as usize;

        log::debug!(
            "Demand Paging: {virtual_addr:#X} -> {:#X}",
            physical_frame.start_address()
        );

        let mut active = PageTable::active();
        active.mapper_mut().map_to(
            VirtualAddress(virtual_addr as _),
            physical_frame,
            PageTableEntryFlags::WRITABLE
                | PageTableEntryFlags::USER_ACCESSIBLE
                | PageTableEntryFlags::NO_EXECUTE,
            &mut *memory::lock_global_frame_allocator(),
        );

        unsafe {
            core::ptr::write_bytes(
                (physical_frame.start_address() + hhdm_offset) as *mut u8,
                0,
                crate::memory::PAGE_SIZE,
            )
        };

        return;
    }

    panic!(
        "\nEXCEPTION: PAGE FAULT while accessing {virtual_addr:#x}\nError code: {:?}\n{:#?}",
        unsafe { PageFaultErrorCode::from_bits(error_code as _).unwrap_unchecked() },
        stack_frame
    );
}

pub extern "C" fn double_fault_handler(stack_frame: &ExceptionStackFrameWithError) -> ! {
    let error_code = stack_frame.error_code;
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

pub extern "C" fn invalid_tss_handler(stack_frame: &ExceptionStackFrameWithError) {
    let error_code = stack_frame.error_code;
    let is_external = error_code & 1 != 0;
    let table = (error_code >> 1) & 0b11;
    let index = (error_code >> 3) & 0x1FFF;

    panic!(
        "\nEXCEPTION: INVALID TSS\nError code: {error_code} (external: {is_external}, table: {table}, index: {index})\n{:#?}",
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
    crate::drivers::keyboard::add_scancode(scancode);
    apic::send_eoi();
}
