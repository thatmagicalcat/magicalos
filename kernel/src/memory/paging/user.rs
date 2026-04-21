use crate::{
    arch::interrupts,
    kernel::{self, USER_ENTRY},
    memory::{
        self, Frame,
        paging::{L4, PageTable, PageTableEntryFlags, PhysicalAddress, VirtualAddress},
    },
    scheduler, utils,
};

pub fn create_page_table() -> PhysicalAddress {
    interrupts::without_interrupts(|| {
        let hhdm_offset = kernel::get_hhdm_offset();
        let frame = memory::allocate_frame().expect("oom");
        let ptr = (frame.start_address() + hhdm_offset) as *mut u8;

        unsafe {
            // zero lower half
            core::ptr::write_bytes(ptr, 0, memory::PAGE_SIZE / 2);

            // now we copy first 256 entires from kernel's page table to this newly created page table
            // so our kernel doesn't die as soon as we switch the page table
            let new_pml4: &mut memory::paging::PhysicalPageTable<L4> = &mut *(ptr as *mut _);
            let kernel_pml4 = &*kernel::get_kernel_page_table().p4;

            new_pml4[256..512].copy_from_slice(&kernel_pml4[256..512]);
        }

        scheduler::set_root_page_table(PhysicalAddress(frame.start_address() as _));

        PhysicalAddress(frame.start_address() as _)
    })
}

/// cr3 should point to the newly created page table by the time this function is called
pub fn map_user_entry(f: extern "C" fn()) {
    interrupts::without_interrupts(|| {
        // get physical frame of the function
        let mut pt = PageTable::active();
        let fn_virt_addr = utils::align_down(f as *const () as usize, memory::PAGE_SIZE);
        let fn_phys_addr = pt
            .mapper_mut()
            .translate(VirtualAddress(fn_virt_addr as u64))
            .expect("Function not mapped in active page table");
        let fn_frame = Frame::from_addr(fn_phys_addr.0 as _);

        // map USER_ENTRY -> function physical frame with appropriate flags
        PageTable::active().mapper_mut().map_to(
            USER_ENTRY,
            fn_frame,
            PageTableEntryFlags::WRITABLE | PageTableEntryFlags::USER_ACCESSIBLE,
            &mut *memory::lock_global_frame_allocator(),
        );

        PageTable::active().mapper_mut().map_to(
            VirtualAddress(USER_ENTRY.0 + memory::PAGE_SIZE as u64),
            Frame(fn_frame.0 + 1),
            PageTableEntryFlags::WRITABLE | PageTableEntryFlags::USER_ACCESSIBLE,
            &mut *memory::lock_global_frame_allocator(),
        );
    });
}
