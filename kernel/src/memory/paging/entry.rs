use crate::memory::Frame;

pub const PHYSICAL_ADDRESS_MASK: u64 = 0xFFFFFFFFFF000;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PageTableEntryFlags: u64 {
        const PRESENT         = 1 << 0;
        const WRITABLE        = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH   = 1 << 3;
        const CACHE_DISABLE   = 1 << 4;
        const ACCESSED        = 1 << 5;
        const DIRTY           = 1 << 6;
        const HUGE_PAGE       = 1 << 7;
        const GLOBAL          = 1 << 8;
        const NO_EXECUTE      = 1 << 63;

       /*
        * 9 - 11 & 52 - 62 are available to be used by the OS
        * 12 - 51 physical address
        */
    }
}

fn f(i: *const i32) -> bool {
    if i as usize % 100 == 0 {
        return true;
    }

    false
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, frame: Frame, flags: PageTableEntryFlags) {
        assert!(
            frame.start_address() & !PHYSICAL_ADDRESS_MASK as usize == 0,
            "invalid physical frame address"
        );

        self.0 = frame.start_address() as u64 | flags.bits();
    }

    pub const fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.0)
    }

    pub const fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub const fn set_flags(&mut self, flags: PageTableEntryFlags) {
        self.0 |= flags.bits();
    }

    pub const fn clear_flags(&mut self, flags: PageTableEntryFlags) {
        self.0 &= !flags.bits();
    }

    pub const fn is_present(&self) -> bool {
        (self.0 & PageTableEntryFlags::PRESENT.bits()) != 0
    }

    pub const fn is_huge(&self) -> bool {
        (self.0 & PageTableEntryFlags::HUGE_PAGE.bits()) != 0
    }

    pub fn get_physical_address(&self) -> super::PhysicalAddress {
        (self.0 & PHYSICAL_ADDRESS_MASK).into()
    }

    pub fn get_pointed_frame(&self) -> Option<Frame> {
        self.flags()
            .contains(PageTableEntryFlags::PRESENT)
            .then_some(Frame::from_addr(*self.get_physical_address() as _))
    }
}

impl Default for PageTableEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn page_table_entry_set_and_decode_roundtrip() {
        let mut entry = PageTableEntry::new();
        let frame = Frame(0x1234);
        let flags = PageTableEntryFlags::PRESENT
            | PageTableEntryFlags::WRITABLE
            | PageTableEntryFlags::USER_ACCESSIBLE;

        entry.set(frame, flags);

        assert_eq!(
            entry.get_physical_address().0 as usize,
            frame.start_address()
        );
        assert!(entry.flags().contains(PageTableEntryFlags::PRESENT));
        assert!(entry.flags().contains(PageTableEntryFlags::WRITABLE));
        assert!(entry.flags().contains(PageTableEntryFlags::USER_ACCESSIBLE));
    }

    #[test_case]
    fn page_table_entry_presence_controls_pointed_frame() {
        let mut entry = PageTableEntry::new();
        let frame = Frame(7);

        entry.set(frame, PageTableEntryFlags::WRITABLE);
        assert_eq!(entry.get_pointed_frame(), None);

        entry.set_flags(PageTableEntryFlags::PRESENT);
        assert_eq!(entry.get_pointed_frame(), Some(frame));
    }

    #[test_case]
    fn page_table_entry_flag_mutation_works() {
        let mut entry = PageTableEntry::new();
        let frame = Frame(11);

        entry.set(frame, PageTableEntryFlags::PRESENT);
        entry.set_flags(PageTableEntryFlags::WRITABLE);
        assert!(entry.flags().contains(PageTableEntryFlags::WRITABLE));

        entry.clear_flags(PageTableEntryFlags::WRITABLE);
        assert!(!entry.flags().contains(PageTableEntryFlags::WRITABLE));
    }
}
