use crate::memory::{Frame, FrameAllocator};
use core::arch::asm;

use super::{
    PhysicalAddress, VirtualAddress,
    entry::PageTableEntryFlags,
    table::{ENTRIES_PER_TABLE, L4, PhysicalPageTable},
};

// use crate::memory::{Frame, FrameAllocator};
//
// pub const P4: *mut PageTable<L4> = 0xFFFFFFFFFFFFF000 as *mut _;
//
#[derive(Clone)]
pub struct Mapper {
    pub(crate) p4: *mut PhysicalPageTable<L4>,
}

impl Mapper {
    pub const fn new(p4: *mut PhysicalPageTable<L4>) -> Self {
        Self { p4 }
    }

    pub const fn as_ref(&self) -> &'static PhysicalPageTable<L4> {
        unsafe { &*self.p4 }
    }

    pub const fn as_mut(&mut self) -> &mut PhysicalPageTable<L4> {
        unsafe { &mut *self.p4 }
    }

    pub fn translate(&mut self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        let p3 = self.as_mut().next_table(virt_addr.p4_idx() as _);
        let huge_pages = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[virt_addr.p3_idx()];

                // 1 GiB page?!?! what the fuck?
                if let Some(start_frame) = p3_entry.get_pointed_frame()
                    && p3_entry.flags().contains(PageTableEntryFlags::HUGE_PAGE)
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
                        && p2_entry.flags().contains(PageTableEntryFlags::HUGE_PAGE)
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
            .and_then(|p1| p1[virt_addr.p1_idx()].get_pointed_frame())
            .or_else(huge_pages)
            .map(|frame| PhysicalAddress(frame.start_address() as u64))
    }

    pub fn map_if_unmapped<A: FrameAllocator>(
        &mut self,
        page: VirtualAddress,
        frame: Frame,
        flags: PageTableEntryFlags,
        allocator: &mut A,
    ) {
        if self.translate(page).is_none() {
            self.map_to(page, frame, flags, allocator);
        }
    }

    pub fn map_to<A: FrameAllocator>(
        &mut self,
        page: VirtualAddress,
        frame: Frame,
        flags: PageTableEntryFlags,
        allocator: &mut A,
    ) {
        // log::trace!("map(): {:#010x} -> {:#010x}, flags = {flags:?}", page.0, frame.start_address());

        let p4 = self.as_mut();
        let p3 = p4.next_table_create(page.p4_idx() as _, flags, allocator);
        let p2 = p3.next_table_create(page.p3_idx() as _, flags, allocator);
        let p1 = p2.next_table_create(page.p2_idx() as _, flags, allocator);

        assert!(p1[page.p1_idx()].is_unused());
        p1[page.p1_idx()].set(frame, flags | PageTableEntryFlags::PRESENT);
    }

    /// Identical to `map_to`, but allocates a frame for you.
    pub fn map<A>(&mut self, page: VirtualAddress, flags: PageTableEntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let frame = allocator.allocate_frame().expect("out of memory");
        self.map_to(page, frame, flags, allocator)
    }

    #[must_use]
    pub fn unmap(&mut self, page: VirtualAddress) -> Frame {
        assert!(self.translate(page).is_some());

        let p1 = self
            .as_mut()
            .next_table_mut(page.p4_idx())
            .and_then(|p3| p3.next_table_mut(page.p3_idx()))
            .and_then(|p2| p2.next_table_mut(page.p2_idx()))
            .expect("mapping code does not support huge pages");
        let frame = p1[page.p1_idx()].get_pointed_frame().unwrap();

        p1[page.p1_idx()].set_unused();

        // TODO: deallocate empty page tables
        // but this is very expensive to do on every unmap...
        // allocator.deallocate_frame(frame);

        crate::flush_tlb!(*page);
        frame
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        kernel,
        memory::{allocate_frame, lock_global_frame_allocator},
    };

    #[test_case]
    fn virtual_to_physical_mapping_works() {
        let virt = 0xdead_beef_u64.into();
        let frame = allocate_frame().expect("out of memory");

        let mapper = kernel::get_kernel_page_table().mapper_mut();

        mapper.map_to(
            virt,
            frame,
            super::PageTableEntryFlags::WRITABLE,
            &mut *lock_global_frame_allocator(),
        );

        let phys = mapper.translate(virt).unwrap();

        assert_eq!(phys.0, frame.start_address() as u64, "Translation failed");

        // test read/write access to the mapped page
        unsafe {
            let ptr = virt.as_mut_ptr::<u64>();

            core::ptr::write_volatile(ptr, 0x1234_5678_9abc_def0);
            let value = core::ptr::read_volatile(ptr);

            assert_eq!(
                value, 0x1234_5678_9abc_def0,
                "Read/write access to mapped page failed"
            );
        }

        // // cleanup
        mapper.unmap(virt);
    }
}
