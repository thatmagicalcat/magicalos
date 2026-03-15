use core::arch::asm;

use super::{
    PhysicalAddress, VirtualAddress,
    entry::EntryFlags,
    table::{ENTRIES_PER_TABLE, L4, PageTable},
};
use crate::memory::{BitmapFrameAllocator, Frame, FrameAllocator};

pub const P4: *mut PageTable<L4> = 0xFFFFFFFFFFFFF000 as *mut _;

pub struct ActivePageTable {
    p4: *mut PageTable<L4>,
}

impl ActivePageTable {
    pub const fn new() -> Self {
        Self { p4: P4 }
    }

    pub const fn as_ref(&self) -> &'static PageTable<L4> {
        unsafe { &*self.p4 }
    }

    pub const fn p4_mut(&mut self) -> &mut PageTable<L4> {
        unsafe { &mut *self.p4 }
    }

    pub fn translate(&mut self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        let p3 = self.p4_mut().next_table(virt_addr.p4_idx() as _);
        let huge_pages = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[virt_addr.p3_idx()];

                // 1 GiB page?!?!
                if let Some(start_frame) = p3_entry.get_pointed_frame()
                    && p3_entry.flags().contains(EntryFlags::HUGE_PAGE)
                {
                    // address must be 1 GiB aligned
                    assert!(start_frame.start_address() % (1 << 30) == 0);
                    return Some(Frame(
                        start_frame.0 + virt_addr.p2_idx() * ENTRIES_PER_TABLE + virt_addr.p1_idx(),
                    ));
                }

                if let Some(p2) = p3.next_table(virt_addr.p3_idx()) {
                    let p2_entry = &p2[virt_addr.p2_idx()];

                    // 2 MiB page
                    if let Some(start_frame) = p2_entry.get_pointed_frame()
                        && p2_entry.flags().contains(EntryFlags::HUGE_PAGE)
                    {
                        // address must be 2 MiB aligned
                        assert!(start_frame.start_address() % (1 << 21) == 0);
                        return Some(Frame(start_frame.0 + virt_addr.p1_idx()));
                    }
                }

                None
            })
        };

        p3.and_then(|p3| p3.next_table(virt_addr.p3_idx()))
            .and_then(|p2| p2.next_table(virt_addr.p2_idx()))
            .and_then(|p1| p1[{ virt_addr.p1_idx() }].get_pointed_frame())
            .or_else(huge_pages)
            .map(|frame| PhysicalAddress(frame.start_address() as u64))
    }

    pub fn map_to(
        &mut self,
        page: VirtualAddress,
        frame: Frame,
        flags: EntryFlags,
        allocator: &mut BitmapFrameAllocator,
    ) {
        let p4 = self.p4_mut();
        let mut p3 = p4.next_table_create(page.p4_idx() as _, allocator);
        let mut p2 = p3.next_table_create(page.p3_idx() as _, allocator);
        let mut p1 = p2.next_table_create(page.p2_idx() as _, allocator);

        assert!(p1[page.p1_idx()].is_unused());
        p1[page.p1_idx()].set(frame, flags | EntryFlags::PRESENT);
    }

    pub fn unmap(&mut self, page: VirtualAddress, allocator: &mut BitmapFrameAllocator) {
        assert!(self.translate(page).is_some());

        let p1 = self
            .p4_mut()
            .next_table_mut(page.p4_idx())
            .and_then(|p3| p3.next_table_mut(page.p3_idx()))
            .and_then(|p2| p2.next_table_mut(page.p2_idx()))
            .expect("mapping code does not support huge pages");
        let frame = p1[page.p1_idx()].get_pointed_frame().unwrap();

        p1[page.p1_idx()].set_unused();

        // TODO: deallocate empty page tables
        // but this is very expensive to do on every unmap...

        allocator.deallocate_frame(frame);

        unsafe { asm!("invlpg [{}]", in(reg) *page, options(nostack, preserves_flags)) };
    }
}
