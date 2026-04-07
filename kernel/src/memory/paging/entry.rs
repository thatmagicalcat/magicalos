use crate::memory::Frame;

pub const PHYSICAL_ADDRESS_MASK: u64 = 0xFFFFFFFFFF000;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EntryFlags: u64 {
        const PRESENT         = 1 << 0;
        const WRITABLE        = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH   = 1 << 3; // PWT
        const CACHE_DISABLE   = 1 << 4; // PCD
        const ACCESSED        = 1 << 5;
        const DIRTY           = 1 << 6;
        const HUGE_PAGE       = 1 << 7;
        const GLOBAL          = 1 << 8;
        const NO_EXECUTE      = 1 << 63;

       /*
        * 9 - 11 are available to be used by the OS
        * 12 - 51 physical address
        * 52 - 62 are available to be used by the OS
        */
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(
            frame.start_address() & !PHYSICAL_ADDRESS_MASK as usize == 0,
            "invalid physical frame address"
        );

        self.0 = frame.start_address() as u64 | flags.bits();
    }

    pub const fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub const fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub const fn set_flags(&mut self, flags: EntryFlags) {
        self.0 |= flags.bits();
    }

    pub const fn clear_flags(&mut self, flags: EntryFlags) {
        self.0 &= !flags.bits();
    }

    pub const fn is_present(&self) -> bool {
        (self.0 & EntryFlags::PRESENT.bits()) != 0
    }

    pub const fn is_huge(&self) -> bool {
        (self.0 & EntryFlags::HUGE_PAGE.bits()) != 0
    }

    pub fn get_physical_address(&self) -> super::PhysicalAddress {
        (self.0 & PHYSICAL_ADDRESS_MASK).into()
    }

    pub fn get_pointed_frame(&self) -> Option<Frame> {
        self.flags()
            .contains(EntryFlags::PRESENT)
            .then_some(Frame::from_addr(*self.get_physical_address() as _))
    }
}
