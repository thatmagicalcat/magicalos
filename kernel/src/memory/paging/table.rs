use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
};

use crate::{
    kernel, limine_requests::HHDM_REQUEST, memory::{
        FrameAllocator,
        paging::{Mapper, PageTableEntryFlags, PhysicalAddress, VirtualAddress},
    }, utils
};

use super::PageTableEntry;

pub const ENTRIES_PER_TABLE: usize = 512;

pub trait Level {}
pub trait TableLevel: Level {
    type NextLevel: Level;
}

pub struct L4;
pub struct L3;
pub struct L2;
pub struct L1;

impl Level for L4 {}
impl Level for L3 {}
impl Level for L2 {}
impl Level for L1 {}

impl TableLevel for L4 {
    type NextLevel = L3;
}

impl TableLevel for L3 {
    type NextLevel = L2;
}

impl TableLevel for L2 {
    type NextLevel = L1;
}

/// The page table structure that is used by the CPU
#[repr(align(4096))]
pub struct PhysicalPageTable<L: Level> {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
    _phantom: PhantomData<L>,
}

impl<L: Level> PhysicalPageTable<L> {
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.set_unused();
        }
    }
}

impl<L: TableLevel> PhysicalPageTable<L> {
    fn next_table_addr(&self, index: usize) -> Option<VirtualAddress> {
        let entry = self[index];

        if entry.is_present() && !entry.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
            return Some(
                entry
                    .get_physical_address()
                    .to_virtual(kernel::get_hhdm_offset() as _),
            );
        }

        None
    }

    pub fn next_table(&self, index: usize) -> Option<&PhysicalPageTable<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { addr.as_ref() })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut PhysicalPageTable<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { addr.as_mut() })
    }

    pub fn next_table_create<A: FrameAllocator>(
        &mut self,
        index: usize,
        additional_flags: PageTableEntryFlags,
        allocator: &mut A,
    ) -> &mut PhysicalPageTable<L::NextLevel> {
        if self.next_table_addr(index).is_none() {
            assert!(
                !self[index].is_huge(),
                "mapping huge pages as a table is not supported"
            );

            let physical_frame = allocator.allocate_frame().expect("OOM");
            self.entries[index].set(
                physical_frame,
                additional_flags | PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
            );

            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap() // safe
    }
}

impl<L: Level> Index<usize> for PhysicalPageTable<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: Level> Index<Range<usize>> for PhysicalPageTable<L> {
    type Output = [PageTableEntry];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: Level> IndexMut<usize> for PhysicalPageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<L: Level> IndexMut<Range<usize>> for PhysicalPageTable<L> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

/// A page table wrapper that provides a safe API to manipulate the page tables.
pub struct PageTable {
    mapper: Mapper,
}

// pub struct InactivePageTable {
//     p4_frame: Frame,
// }
//
// impl InactivePageTable {
//     pub fn new<A: FrameAllocator>(
//         frame: Frame,
//         active_tbl: &mut ActivePageTable,
//         tmp_addr: VirtualAddress,
//         allocator: &mut A,
//     ) -> Self {
//         active_tbl.map_to(
//             tmp_addr,
//             frame,
//             EntryFlags::PRESENT | EntryFlags::WRITABLE,
//             allocator,
//         );
//
//         let table = unsafe { &mut *tmp_addr.as_mut_ptr::<PageTable<L1>>() };
//         table.zero();
//         table[511].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
//
//         // idk what to do with it
//         _ = active_tbl.unmap(tmp_addr);
//
//         Self { p4_frame: frame }
//     }
// }

impl PageTable {
    pub fn active() -> Self {
        Self {
            mapper: Mapper::new(
                utils::read_cr3()
                    .to_virtual(kernel::get_hhdm_offset() as _)
                    .as_mut_ptr(),
            ),
        }
    }

    pub const fn new(mapper: Mapper) -> Self {
        Self { mapper }
    }

    /// Literally calls `Mapper::new` with the given virtual address
    pub const fn from_phys_table_addr(addr: *mut PhysicalPageTable<L4>) -> Self {
        Self {
            mapper: Mapper::new(addr),
        }
    }

    pub fn get_physical_address(&self) -> PhysicalAddress {
        // physical addr = virtual addr - hhdm_offset
        PhysicalAddress(self.mapper.p4 as usize as u64 - kernel::get_hhdm_offset() as u64)
    }

    // The trick:
    // 1. Backup the physical address of active P4 table physical address
    // 2. Map the temporary address to the active P4 table physical address
    // 3. Map 511th entry of the temporary page to the new inactive P4 table physical address
    //    so we can hijack the mapper's map_to method to map the new P4 table
    // 4. Flush the TLP
    // 5. Run the closure `f` with the new mapper
    // 6. Restore the original P4 table mapping
    // 7. Unmap the temporary page
    // pub fn with<F, A>(
    //     &mut self,
    //     table: &InactivePageTable,
    //     tmp_addr: VirtualAddress,
    //     allocator: &mut A,
    //     f: F,
    // ) where
    //     F: FnOnce(&mut Mapper, &mut A),
    //     A: FrameAllocator,
    // {
    //     let backup_frame = unsafe {
    //         let value: u64;
    //         asm! {
    //             "mov {}, cr3",
    //             out(reg) value,
    //             options(nomem, nostack, preserves_flags)
    //         };
    //
    //         Frame::from_addr((value & PHYSICAL_ADDRESS_MASK) as _)
    //     };
    //
    //     self.mapper.map_to(
    //         tmp_addr,
    //         backup_frame,
    //         EntryFlags::PRESENT | EntryFlags::WRITABLE,
    //         allocator,
    //     );
    //
    //     self.mapper.as_mut()[511].set(table.p4_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
    //     flush_tlb!();
    //
    //     f(self, allocator);
    //
    //     let p4_tbl = unsafe { &mut *(*tmp_addr as *mut PageTable<L1>) };
    //     p4_tbl[511].set(backup_frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
    //     flush_tlb!();
    //
    //    _ = self.mapper.unmap(tmp_addr);
    // }

    // pub fn switch_table(&mut self, inactive_table: InactivePageTable) -> InactivePageTable {
    //     let old_table = InactivePageTable {
    //         p4_frame: Frame::from_addr((utils::read_cr3().0 & PHYSICAL_ADDRESS_MASK) as _),
    //     };
    //
    //     // invalidates TLB, no need to flush
    //     utils::write_cr3(inactive_table.p4_frame.start_address() as _);
    //
    //     old_table
    // }

    pub const fn mapper(&self) -> &Mapper {
        &self.mapper
    }

    pub const fn mapper_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl Deref for PageTable {
    type Target = Mapper;

    fn deref(&self) -> &Self::Target {
        self.mapper()
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mapper_mut()
    }
}
