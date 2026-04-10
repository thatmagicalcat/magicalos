//! source: https://wiki.osdev.org/Global_Descriptor_Table

use bit_field::BitField;
use spin::Once;

use crate::interrupts::{self, TSS, Tss};

const GDT_SIZE: usize = 7;
static GDT: Once<[GdtEntry; GDT_SIZE]> = Once::new();

fn gdt() -> &'static [GdtEntry; GDT_SIZE] {
    GDT.call_once(|| {
        let (tss_low, tss_high) = GdtEntry::tss_seg(&TSS.lock());

        [
            GdtEntry::new(),
            GdtEntry::kernel_code_seg(),
            GdtEntry::kernel_data_seg(),
            GdtEntry::ring3_data_seg(),
            GdtEntry::ring3_code_seg(),
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

    log::info!("Loading GDT: {:x?}", gdtr);
    interrupts::without_interrupts(|| {
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

                "mov ax, 40", // offset (in bytes) of tss segment
                "ltr ax",

                in(reg) &gdtr,
                out(reg) _,
                out("ax") _,

                options(readonly, preserves_flags)
            }
        };
    });
}

#[repr(C, packed)]
#[derive(Debug)]
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct GdtEntry(u64);

#[rustfmt::skip]
#[allow(dead_code)]
impl GdtEntry {
    const fn new() -> Self {
        Self(0)
    }

    // --- getters ---
    fn limit_low(&self) -> u16 { self.0.get_bits(0..16) as u16 }
    fn base_low(&self) -> u32  { self.0.get_bits(16..40) as u32 }
    /// Types:
    /// 0x1 - 16-bit TSS (Available)
    /// 0x2 - LDT
    /// 0x3 - 16-bit TSS (Busy)
    /// 0x9 - 64-bit TSS (Available)
    /// 0xB - 64-bit TSS (Busy)
    fn system_segment_type(&self) -> u8 { self.0.get_bits(40..44) as u8 }
    fn accessed(&self) -> bool { self.0.get_bit(40) }
    fn read_write(&self) -> bool { self.0.get_bit(41) }
    fn conforming_expand_down(&self) -> bool { self.0.get_bit(42) }
    fn is_code(&self) -> bool { self.0.get_bit(43) }
    fn is_code_data_segment(&self) -> bool { self.0.get_bit(44) }
    fn dpl(&self) -> u8 { self.0.get_bits(45..47) as u8 }
    fn present(&self) -> bool { self.0.get_bit(47) }
    fn limit_high(&self) -> u8 { self.0.get_bits(48..52) as u8 }
    fn available(&self) -> bool { self.0.get_bit(52) }
    fn long_mode(&self) -> bool { self.0.get_bit(53) }
    fn big(&self) -> bool { self.0.get_bit(54) }
    fn gran(&self) -> bool { self.0.get_bit(55) }
    fn base_high(&self) -> u8 { self.0.get_bits(56..64) as u8 }

    // --- setters ---
    fn set_limit_low(&mut self, val: u16) -> &mut Self { self.0.set_bits(0..16, val as u64); self }
    fn set_base_low(&mut self, val: u32) -> &mut Self { self.0.set_bits(16..40, val as u64); self }
    /// Types:
    /// 0x1 - 16-bit TSS (Available)
    /// 0x2 - LDT
    /// 0x3 - 16-bit TSS (Busy)
    /// 0x9 - 64-bit TSS (Available)
    /// 0xB - 64-bit TSS (Busy)
    fn set_system_segment_type(&mut self, ty: u8) -> &mut Self { self.0.set_bits(40..44, ty as u64); self }
    fn set_accessed(&mut self) -> &mut Self { self.0.set_bit(40, true); self }
    fn set_read_write(&mut self) -> &mut Self { self.0.set_bit(41, true); self }
    fn set_conforming_expand_down(&mut self) -> &mut Self { self.0.set_bit(42, true); self }
    fn set_code(&mut self) -> &mut Self { self.0.set_bit(43, true); self }
    fn set_code_data_segment(&mut self) -> &mut Self { self.0.set_bit(44, true); self }
    fn set_dpl(&mut self, val: u8) -> &mut Self { self.0.set_bits(45..47, val as u64); self }
    fn set_present(&mut self) -> &mut Self { self.0.set_bit(47, true); self }
    fn set_limit_high(&mut self, val: u8) -> &mut Self { self.0.set_bits(48..52, val as u64); self }
    fn set_available(&mut self) -> &mut Self { self.0.set_bit(52, true); self }
    fn set_long_mode(&mut self) -> &mut Self { self.0.set_bit(53, true); self }
    fn set_big(&mut self) -> &mut Self { self.0.set_bit(54, true); self }
    fn set_gran(&mut self) -> &mut Self { self.0.set_bit(55, true); self }
    fn set_base_high(&mut self, val: u8) -> &mut Self { self.0.set_bits(56..64, val as u64); self }

    /// Export the final raw u64
    const fn bits(&self) -> u64 {
        self.0
    }

    fn kernel_code_seg() -> Self {
        *Self::new()
            .set_present() // Must be set
            .set_dpl(0) // RING 0
            .set_code_data_segment() // 1 for code/data seg
            .set_code() // executable bit
            .set_gran() // 4KiB granularity
            .set_read_write() // Code segs are readable
            .set_long_mode() // 64-bit code segment
    }

    fn kernel_data_seg() -> Self {
        *Self::new()
            .set_present() // Must be set
            .set_dpl(0) // RING 0
            .set_code_data_segment() // 1 for code/data seg
            .set_read_write() // Data segs are writable
    }

    fn ring3_data_seg() -> Self {
        *Self::new()
            .set_present()
            .set_limit_low(0xFFFF)
            .set_limit_high(0xF)
            .set_read_write()
            .set_code_data_segment()
            .set_dpl(3) // Ring 3
            .set_available()
            .set_gran() // 4KiB granularity
    }

    fn ring3_code_seg() -> Self {
        *Self::ring3_data_seg()
            .set_long_mode()
            .set_code() // executable bit
    }

    fn tss_seg(tss: &Tss) -> (Self, Self) {
        let ptr = tss as *const _ as u64;
        let limit = (size_of::<Tss>() - 1) as u32;

        // -- low 64 bits ---
        let low = *Self::new()
            .set_limit_low(limit as _)
            .set_limit_high((limit.get_bits(16..20)) as _)
            .set_base_low(ptr.get_bits(0..24) as _)
            .set_base_high(ptr.get_bits(24..32) as _)
            .set_present()
            .set_system_segment_type(0x9); // 64-bit Available TSS

        let mut high = GdtEntry::new();
        high.0.set_bits(0..32, ptr.get_bits(32..64));

        (low, high)
    }
}

impl Default for GdtEntry {
    fn default() -> Self {
        Self::new()
    }
}
