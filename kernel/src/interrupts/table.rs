use core::arch::asm;
use bit_field::BitField;
use crate::utils::read_cs;

pub struct Idt(pub [Entry; 256]);

impl Idt {
    pub fn new() -> Idt {
        Idt([Entry::missing(); _])
    }

    pub fn set_handler(&mut self, entry: u8, handler: HandlerFn) {
        self.0[entry as usize] = Entry::new(SegmentSelector(read_cs()), handler);
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Entry>() * self.0.len() - 1) as u16,
            base: self.0.as_ptr() as u64,
        };

        unsafe { asm!("lidt [{}]", in(reg) &ptr, options(readonly, nostack, preserves_flags)) };
    }
}

bitflags::bitflags! {
    struct PrivilegeLevel: u16 {
        const RING0 = 0;
        const RING1 = 1;
        const RING2 = 2;
        const RING3 = 3;
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    #[inline]
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> SegmentSelector {
        Self((index << 3) | rpl.bits())
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
            gdt_selector: SegmentSelector::new(0, PrivilegeLevel::RING0),
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
