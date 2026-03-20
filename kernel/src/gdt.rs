//! source: https://wiki.osdev.org/Global_Descriptor_Table

use bit_field::BitField;
use spin::Once;

use crate::interrupts::{Tss, TSS};

const GDT_SIZE: usize = 5;
static GDT: Once<[GdtEntry; GDT_SIZE]> = Once::new();

fn gdt() -> &'static [GdtEntry; GDT_SIZE] {
    GDT.call_once(|| {
        let (tss_low, tss_high) = GdtEntry::tss_seg(&TSS);

        [
            GdtEntry::empty(),
            GdtEntry::kernel_code_seg(),
            GdtEntry::new_data(),
            tss_low,
            tss_high,
        ]
    })
}

// source: https://wiki.osdev.org/GDT_Tutorial#:~:text=reloadSegments%3A%20%3B%20Reload,RET
pub fn init() {
    let gdtr = GdtR {
        size: (core::mem::size_of::<[GdtEntry; GDT_SIZE]>() - 1) as _,
        base: gdt().as_ptr() as _,
    };

    unsafe {
        core::arch::asm! {
            "lgdt [{0}]",

            "push 8",     // offset (in bytes) of code segment
            "lea {1}, [2f]",
            "push {1}",
            "retfq",

            "2:",
            "mov ax, 16", // offset (in bytes) of data segment
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",

            "mov ax, 24", // offset (in bytes) of tss segment
            "ltr ax",

            in(reg) &gdtr,
            out(reg) _,
            out("ax") _,

            options(readonly, preserves_flags)
        }
    };
}

#[repr(C, packed)]
struct GdtR {
    /// size of table in bytes - 1
    pub size: u16,
    /// virtual address to the table
    pub base: u64,
}

bitflags::bitflags! {
    struct DescriptorFlags: u64 {
        const CONFORMING        = 1 << 42;
        const EXECUTABLE        = 1 << 43;
        const USER_SEGMENT      = 1 << 44;
        const PRESENT           = 1 << 47;
        const LONG_MODE         = 1 << 53;
        const CS_READABLE       = 1 << 41;
        const DS_WRITABLE       = 1 << 41;
    }
}

struct GdtEntry(pub u64);
impl GdtEntry {
    const fn empty() -> Self {
        Self(0)
    }

    fn kernel_code_seg() -> Self {
        Self(
            (DescriptorFlags::USER_SEGMENT
                | DescriptorFlags::PRESENT
                | DescriptorFlags::CS_READABLE
                | DescriptorFlags::EXECUTABLE
                | DescriptorFlags::LONG_MODE)
                .bits(),
        )
    }

    fn new_data() -> Self {
        Self(
            (DescriptorFlags::USER_SEGMENT
                | DescriptorFlags::PRESENT
                | DescriptorFlags::DS_WRITABLE)
                .bits(),
        )
    }

    fn tss_seg(tss: &Tss) -> (Self, Self) {
        let ptr = tss as *const _ as u64;

        let mut low = DescriptorFlags::PRESENT.bits();
        low.set_bits(16..40, ptr.get_bits(0..24));
        low.set_bits(56..64, ptr.get_bits(24..32));
        low.set_bits(0..16, (size_of::<Tss>() - 1) as u64);
        low.set_bits(40..44, 0b1001);

        let mut high = 0;
        high.set_bits(0..32, ptr.get_bits(32..64));

        (Self(low), Self(high))
    }
}
