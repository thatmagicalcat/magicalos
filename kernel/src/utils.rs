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
