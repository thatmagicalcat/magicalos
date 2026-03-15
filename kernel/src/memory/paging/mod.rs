use bitflags::bitflags;

use super::{Frame, PAGE_SIZE};

mod table;

const ENTRIES_PER_TABLE: usize = 512;
const PHYSICAL_ADDRESS_MASK: u64 = 0xFFFFFFFFFF000;

// pub type VirtAddr = usize;
// pub type PhysAddr = usize;

#[repr(transparent)]
pub struct PhysAddr(usize);

#[repr(transparent)]
pub struct VirtAddr(usize);

#[rustfmt::skip]
impl VirtAddr {
    pub fn p4_idx(&self) -> usize { self.0 >> 27 & 0o777 }
    pub fn p3_idx(&self) -> usize { self.0 >> 18 & 0o777 }
    pub fn p2_idx(&self) -> usize { self.0 >> 9  & 0o777 }
    pub fn p1_idx(&self) -> usize { self.0 >> 0  & 0o777 }

    pub fn containing_frame(&self) -> Frame {
        const MIN: usize = 1 << 47;
        const MAX: usize = !0 << 47;

        assert!(
            self.0 < MIN || self.0 >= MAX,
            "virtual address out of range"
        );

        Frame::from_addr(self.0)
    }
}

bitflags! {
    pub struct EntryFlags: u64 {
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
        * 9 - 11 are available to be used by the OS
        * 12 - 51 physical address
        * 52 - 62 are available to be used by the OS
        */
    }
}

pub struct PageTableEntry(u64);
impl PageTableEntry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            Some(Frame::from_addr((self.0 & PHYSICAL_ADDRESS_MASK) as usize))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address() & !PHYSICAL_ADDRESS_MASK as usize == 0);
        self.0 = frame.start_address() as u64 | flags.bits();
    }
}
