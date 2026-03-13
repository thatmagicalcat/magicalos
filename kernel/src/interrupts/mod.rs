use core::arch::{asm, naked_asm};

use bit_field::BitField;
use bitflags::bitflags;
use lazy_static::lazy_static;
use x86_64::{
    PrivilegeLevel, instructions::segmentation, registers::segmentation::Segment,
    structures::gdt::SegmentSelector,
};

#[macro_use]
mod macros;

use crate::{
    println,
    vga_buffer::{Color, WRITER},
};

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

lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(0, exception_handler!(divide_by_zero_handler));
        idt.set_handler(14, exception_handler_with_error_code!(page_fault_handler));

        idt
    };
}

extern "C" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) -> ! {
    println!("\nEXCEPTION: DIVIDE BY ZERO\n{stack_frame:#?}");
    loop {}
}

extern "C" fn page_fault_handler(stack_frame: &ExceptionStackFrame, error_code: u64) -> ! {
    unsafe {
        println!(
            "\nEXCEPTION: PAGE FAULT while accessing {:#x}\nError code: {:?}\n{:#?}",
            x86_64::registers::control::Cr2::read().unwrap_unchecked(),
            PageFaultErrorCode::from_bits(error_code).unwrap_unchecked(),
            stack_frame
        );
    }

    loop {}
}

pub struct Idt(pub [Entry; 256]);

impl Idt {
    pub fn new() -> Idt {
        Idt([Entry::missing(); _])
    }

    pub fn set_handler(&mut self, entry: u8, handler: HandlerFn) {
        self.0[entry as usize] = Entry::new(segmentation::CS::get_reg(), handler);
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Entry>() * self.0.len() - 1) as u16,
            base: self.0.as_ptr() as u64,
        };

        unsafe { asm!("lidt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags)) };
    }
}

#[repr(C, packed)]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

pub type HandlerFn = extern "C" fn() -> !;

impl Entry {
    pub fn new(gdt_selector: SegmentSelector, handler: HandlerFn) -> Self {
        let ptr = handler as usize;
        Self {
            pointer_low: ptr as u16,
            gdt_selector,
            options: EntryOptions::new(),
            pointer_middle: (ptr >> 16) as u16,
            pointer_high: (ptr >> 32) as u32,
            reserved: 0,
        }
    }

    fn missing() -> Entry {
        Entry {
            pointer_low: 0,
            gdt_selector: SegmentSelector::new(0, PrivilegeLevel::Ring0),
            options: EntryOptions::minimal(),
            pointer_middle: 0,
            pointer_high: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EntryOptions(u16);

impl EntryOptions {
    fn minimal() -> Self {
        let mut options = 0;
        options.set_bits(9..12, 0b111); // 'must-be-one' bits
        EntryOptions(options)
    }

    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present(true).disable_interrupts(true);
        options
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    pub fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bit(8, !disable);
        self
    }

    pub fn set_privilege_level(&mut self, dpl: u16) -> &mut Self {
        self.0.set_bits(13..15, dpl);
        self
    }

    pub fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0.set_bits(0..3, index);
        self
    }
}

pub fn init() {
    IDT.load();
}

#[derive(Debug)]
#[repr(C)]
struct ExceptionStackFrame {
    instr_ptr: usize,
    code_segment: usize,
    cpu_flags: usize,
    stack_ptr: usize,
    stack_segment: usize,
}
