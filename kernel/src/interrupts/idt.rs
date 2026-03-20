use core::arch::naked_asm;

use spin::Lazy;

use crate::memory::PAGE_SIZE;

use super::{handlers::*, table::Idt};

pub const DIVIDE_BY_ZERO: u8 = 0;
pub const PAGE_FAULT: u8 = 14;
pub const BREAKPOINT: u8 = 3;
pub const DOUBLE_FAULT: u8 = 8;
pub const SPURIOUS_INTERRUPT: u8 = 255;
pub const APIC_TIMER: u8 = 32;

pub const IST_STACK_SIZE: usize = PAGE_SIZE;
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

#[repr(align(16))]
struct Stack([u8; IST_STACK_SIZE]);
const IST_TABLE_SIZE: usize = 7; // 7 entries
static mut DOUBLE_FAULT_STACK: Stack = Stack([0; IST_STACK_SIZE]);

pub static TSS: Lazy<Tss> = Lazy::new(|| Tss::default().init());

lazy_static::lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(DIVIDE_BY_ZERO, exception_handler!(divide_by_zero_handler));
        idt.set_handler(PAGE_FAULT, exception_handler_with_error_code!(page_fault_handler));
        idt.set_handler(BREAKPOINT, exception_handler!(breakpoint_handler));
        idt.set_handler(SPURIOUS_INTERRUPT, exception_handler!(spurious_interrupt_handler));
        idt.set_handler(APIC_TIMER, exception_handler!(apic_timer_handler));
        idt.set_handler(DOUBLE_FAULT, exception_handler_with_error_code!(double_fault_handler))
            .options_mut()
            .set_stack_index(1);

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

        self.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack_top;
        self
    }
}
