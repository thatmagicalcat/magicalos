use core::arch::naked_asm;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::{exception_handler, exception_handler_with_error_code, memory::PAGE_SIZE};

use super::{handlers::*, table::Idt};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum InterruptEntryType {
    DivideByZero = 0,
    DeviceNotAvailable = 7,
    PageFault = 14,
    Breakpoint = 3,
    InvalidOpcode = 6,
    DoubleFault = 8,
    InvalidTSS = 10,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    ApicTimer = 32,
    Keyboard = 33,
    Reschedule = 34,
    SpuriousInterrupt = 255,
}

pub const IST_STACK_SIZE: usize = PAGE_SIZE;
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;

#[repr(align(16))]
struct Stack([u8; IST_STACK_SIZE]);
const IST_TABLE_SIZE: usize = 7; // 7 entries

static mut DOUBLE_FAULT_STACK: Stack = Stack([0; IST_STACK_SIZE]);
static mut PAGE_FAULT_STACK: Stack = Stack([0; IST_STACK_SIZE]);

lazy_static! {
    pub static ref TSS: Mutex<Tss> = Mutex::new(Tss::default().init());
}

lazy_static::lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        use InterruptEntryType::*;

        idt.set_handler(DivideByZero, exception_handler!(divide_by_zero_handler));
        idt.set_handler(DeviceNotAvailable, exception_handler!(device_not_available));
        idt.set_handler(Breakpoint, exception_handler!(breakpoint_handler));
        idt.set_handler(InvalidOpcode, exception_handler!(invalid_opcode));
        idt.set_handler(StackSegmentFault, exception_handler_with_error_code!(stack_segment_fault));
        idt.set_handler(SpuriousInterrupt, exception_handler!(spurious_interrupt_handler));
        idt.set_handler(InvalidTSS, exception_handler_with_error_code!(invalid_tss_handler));
        idt.set_handler(Keyboard, exception_handler!(keyboard_handler));
        idt.set_handler(GeneralProtectionFault, exception_handler_with_error_code!(general_protection_fault_handler));
        idt.set_handler(ApicTimer, exception_handler!(acpi_timer_interrupt));
        idt.set_handler(PageFault, exception_handler_with_error_code!(page_fault_handler))
            .options_mut()
            .set_stack_index(PAGE_FAULT_IST_INDEX + 1);
        idt.set_handler(DoubleFault, exception_handler_with_error_code!(double_fault_handler))
            .options_mut()
            .set_stack_index(DOUBLE_FAULT_IST_INDEX + 1);

        idt
    };
}

#[derive(Default)]
#[repr(C, packed)]
pub struct Tss {
    _reserved1: u32,
    privilege_stack_table: [u64; 3],
    _reserved2: u64,
    interrupt_stack_table: [u64; IST_TABLE_SIZE],
    _reserved3: u64,
    _reserved4: u16,
    io_map_base_addr: u16,
}

impl Tss {
    // SAFETY: This function must only be called once, and the returned Tss must not be modified
    // after initialization.
    #[allow(static_mut_refs)]
    pub fn init(mut self) -> Self {
        let double_fault_stack_top =
            unsafe { DOUBLE_FAULT_STACK.0.as_ptr().add(IST_STACK_SIZE) as u64 };
        let page_fault_stack_top =
            unsafe { PAGE_FAULT_STACK.0.as_ptr().add(IST_STACK_SIZE) as u64 };

        self.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack_top;
        self.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = page_fault_stack_top;
        self
    }
}

pub fn set_kernel_stack(stack: u64) {
    TSS.lock().privilege_stack_table[0] = stack;
}
