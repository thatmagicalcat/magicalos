use crate::{
    flush_tlb, kernel,
    memory::{
        self,
        paging::{
            L1, L4, PageTable, PageTableEntryFlags, PhysicalAddress, PhysicalPageTable, TableLevel,
        },
    },
};

/// Returns the physical address of newly created page table of the forked process
pub fn fork_parent_page_table() -> PhysicalAddress {
    let hhdm_offset = kernel::get_hhdm_offset();
    let child_p4_frame = memory::allocate_frame().expect("oom");
    let child_p4 = unsafe {
        &mut *((child_p4_frame.start_address() + hhdm_offset) as *mut PhysicalPageTable<L4>)
    };

    let kernel_pml4 = unsafe { &*kernel::get_kernel_page_table().p4 };
    child_p4[256..512].copy_from_slice(&kernel_pml4[256..512]);

    unsafe { core::ptr::write_bytes(child_p4[0..256].as_mut_ptr(), 0, 256) };

    let mut active = PageTable::active();
    let parent_p4 = active.mapper_mut().as_mut();

    for i in 0..256 {
        if parent_p4[i].is_present() {
            L4::copy_table_level(child_p4, parent_p4, i);
        }
    }

    flush_tlb![];

    child_p4_frame.start_address().into()
}

trait CopyTableLevel<L> {
    fn copy_table_level(
        child: &mut PhysicalPageTable<L>,
        parent: &mut PhysicalPageTable<L>,
        idx: usize,
    );
}

impl<L> CopyTableLevel<L> for L
where
    L: TableLevel,
    L::NextLevel: CopyTableLevel<L::NextLevel>,
{
    fn copy_table_level(
        child: &mut PhysicalPageTable<L>,
        parent: &mut PhysicalPageTable<L>,
        idx: usize,
    ) {
        let entry = parent[idx];
        if !entry.is_present() {
            return;
        }

        let next_child_table =
            child.next_table_create(idx, &mut *memory::lock_global_frame_allocator());
        let next_parent_table: &mut PhysicalPageTable<L::NextLevel> =
            unsafe { &mut *entry.get_physical_address().to_virtual().as_mut_ptr() };

        for next_idx in 0..512 {
            if !next_parent_table[next_idx].is_present() {
                <L as TableLevel>::NextLevel::copy_table_level(
                    next_child_table,
                    next_parent_table,
                    next_idx,
                );
            }
        }
    }
}

impl CopyTableLevel<L1> for L1 {
    fn copy_table_level(
        child: &mut PhysicalPageTable<L1>,
        parent: &mut PhysicalPageTable<L1>,
        idx: usize,
    ) {
        let entry = parent[idx];
        if !entry.is_present() {
            return;
        }

        let mut flags = entry.flags();
        let frame = entry.get_physical_address();

        if flags.contains(PageTableEntryFlags::WRITABLE) {
            flags.remove(PageTableEntryFlags::WRITABLE);
            flags.insert(PageTableEntryFlags::COPY_ON_WRITE);

            parent[idx].set_flags(flags);
        }

        child[idx].set(frame.into(), flags);
        memory::lock_global_frame_allocator().inc_ref_count(frame.into());
    }
}
