use crate::io::apic::DivideConfig;

pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    } else if align == 0 {
        addr
    } else {
        panic!("`align` must be a power of 2");
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

pub fn read_cs() -> u16 {
    let cs: u16;
    unsafe {
        core::arch::asm!(
            "mov {0:x}, cs",
            out(reg) cs,
            options(nomem, nostack, preserves_flags)
        );
    }

    cs
}

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;

    unsafe {
        core::arch::asm! {
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        };
    }

    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    unsafe {
        core::arch::asm! {
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        };
    }
}

pub struct LApicConfig {
    pub initial_count: u32,
    pub divide_config: DivideConfig,
}

pub fn duration_to_timer_config(duration_ns: u64, lapic_freq_hz: u64) -> Option<LApicConfig> {
    let freq_u128 = lapic_freq_hz as u128;
    let duration_u128 = duration_ns as u128;
    let one_second_ns = 1_000_000_000_u128;

    for (divider, dcr_value) in DivideConfig::ASCENDING.iter() {
        let ticks = (freq_u128 * duration_u128) / (one_second_ns * (*divider as u128));

        // check if it fits safely inside the 32-bit hardware register
        if ticks <= u32::MAX as u128 {
            return Some(LApicConfig {
                initial_count: ticks as u32,
                divide_config: *dcr_value,
            });
        }
    }

    // duration is too massive for the LAPIC hardware
    None
}

