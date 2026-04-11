use core::arch::asm;

use crate::{
    bus::{port::Port},
    arch::apic,
    kernel::USER_ENTRY,
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

pub extern "C" fn breakpoint_handler(stack_frame: &ExceptionStackFrame) {
    log::warn!(
        "\nEXCEPTION: BREAKPOINT at {:#X}\n{:#?}",
        stack_frame.rip,
        stack_frame
    );
}

pub extern "C" fn stack_segment_fault(stack_frame: &ExceptionStackFrame, error_code: u64) {
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

pub extern "C" fn page_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) {
    let mut virtual_addr: usize;

    unsafe {
        asm! {
            "mov {}, cr2",
            out(reg) virtual_addr,
            options(nomem, nostack, preserves_flags)
        };
    }

    // 64 KiB
    const MAX_STACK_SIZE: usize = 64 * 1024;
    const USER_STACK_TOP: usize = USER_ENTRY.0 as usize + 0x400000;

    if (USER_STACK_TOP - MAX_STACK_SIZE..USER_STACK_TOP).contains(&virtual_addr) {
        virtual_addr = utils::align_down(virtual_addr, memory::PAGE_SIZE) as _;
        let physical_addr = memory::allocate_frame().expect("oom");

        log::trace!(
            "Map {virtual_addr:#X} -> {:#X}",
            physical_addr.start_address()
        );

        let mut active = PageTable::active();
        active.mapper_mut().map(
            VirtualAddress(virtual_addr as _),
            PageTableEntryFlags::WRITABLE
                | PageTableEntryFlags::USER_ACCESSIBLE
                | PageTableEntryFlags::NO_EXECUTE,
            &mut *memory::lock_global_frame_allocator(),
        );

        unsafe { core::ptr::write_bytes(virtual_addr as *mut u8, 0, crate::memory::PAGE_SIZE) };
        return;
    }

    panic!(
        "\nEXCEPTION: PAGE FAULT while accessing {virtual_addr:#x}\nError code: {:?}\n{:#?}",
        unsafe { PageFaultErrorCode::from_bits(error_code).unwrap_unchecked() },
        stack_frame
    );
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

pub extern "C" fn invalid_tss_handler(stack_frame: &ExceptionStackFrame, error_code: u64) {
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
    crate::async_rt::keyboard::add_scancode(scancode);
    apic::send_eoi();
}
