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

pub fn enable_interrupts() {
    unsafe { core::arch::asm!("sti", options(nomem, nostack, preserves_flags)) };
}
