use bitflags::bitflags;

use crate::{interrupts, port::Port, utils::rdmsr};

pub const PIC1: u16 = 0x20;
pub const PIC2: u16 = 0xA0;
pub const PIC1_COMMAND: u16 = PIC1;
pub const PIC1_DATA: u16 = PIC1 + 1;
pub const PIC2_COMMAND: u16 = PIC2;
pub const PIC2_DATA: u16 = PIC2 + 1;

pub const IA32_APIC_BASE_MSR: u32 = 0x1B;

/// Assuming LAPIC is identity mapped
pub const LAPIC_PHYSICAL_ADDRESS_DEFAULT: usize = 0xFEE0_0000;
pub const LAPIC_ENABLE_BIT: u64 = 1 << 11;
pub const LAPIC_EOI_REG_OFFSET: usize = 0xB0;
pub const LAPIC_SIVR_REG_OFFSET: usize = 0xF0;
pub const LAPIC_ID_REG_OFFSET: usize = 0x20;
pub const LAPIC_DIVIDE_CONFIG_REG_OFFSET: usize = 0x3E0;
pub const LAPIC_INITIAL_COUNT_REG_OFFSET: usize = 0x380;
pub const LAPIC_LVT_TIMER_REG_OFFSET: usize = 0x320;

bitflags! {
    pub struct DivideConfig: u32 {
        const DIVIDE_BY_2   = 0b000;
        const DIVIDE_BY_4   = 0b001;
        const DIVIDE_BY_8   = 0b010;
        const DIVIDE_BY_16  = 0b011;
        const DIVIDE_BY_32  = 0b100;
        const DIVIDE_BY_64  = 0b101;
        const DIVIDE_BY_128 = 0b110;
        const DIVIDE_BY_1   = 0b111;
    }

    pub struct LvtTimerMode: u32 {
        const ONESHOT  = 0;
        const PERIODIC = 1 << 17;
    }
}

const fn register_ptr(offset: usize) -> *mut u32 {
    (LAPIC_PHYSICAL_ADDRESS_DEFAULT + offset) as *mut _
}

pub fn write(offset: usize, value: u32) {
    unsafe { register_ptr(offset).write_volatile(value) };
}

pub fn read(offset: usize) -> u32 {
    unsafe { register_ptr(offset).read_volatile() }
}

pub fn send_eoi() {
    write(LAPIC_EOI_REG_OFFSET, 0);
}

pub fn init() {
    pic_disable();
    lapic_enable();
}

pub fn init_timer(divide_config: DivideConfig, initial_count: u32, mode: LvtTimerMode) {
    write(LAPIC_DIVIDE_CONFIG_REG_OFFSET, divide_config.bits());
    write(LAPIC_INITIAL_COUNT_REG_OFFSET, initial_count);
    write(
        LAPIC_LVT_TIMER_REG_OFFSET,
        mode.bits() | interrupts::APIC_TIMER as u32,
    );
}

fn pic_disable() {
    unsafe {
        u8::write_to_port(PIC1_DATA, 0xFF);
        u8::write_to_port(PIC2_DATA, 0xFF);
    }
}

fn lapic_enable() {
    // if the APIC is already enabled, do nothing
    let mut msr_value = unsafe { rdmsr(IA32_APIC_BASE_MSR) };

    // if not enabled, try to enable the APIC
    if msr_value & LAPIC_ENABLE_BIT == 0 {
        // try to enable the APIC by setting the enable bit in the MSR
        msr_value |= LAPIC_ENABLE_BIT;
        unsafe { crate::utils::wrmsr(IA32_APIC_BASE_MSR, msr_value) };

        // verify that the APIC is now enabled
        let new_msr_value = unsafe { rdmsr(IA32_APIC_BASE_MSR) };
        if new_msr_value & LAPIC_ENABLE_BIT == 0 {
            panic!("Failed to enable the Local APIC");
        }
    }

    write(LAPIC_SIVR_REG_OFFSET, 0x100 | 0xFF);
}
