use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use super::entry::{EntryFlags, PageTableEntry};
use crate::memory::{BitmapFrameAllocator, FrameAllocator};

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

pub struct PageTable<L: Level> {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
    _phantom: PhantomData<L>,
}

impl<L: Level> PageTable<L> {
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.set_unused();
        }
    }
}

impl<L: TableLevel> PageTable<L> {
    /// formula:
    /// next_table_addr = table_addr << 9 | index << 12
    fn next_table_addr(&self, index: usize) -> Option<usize> {
        let flags = self[index].flags();
        if flags.contains(EntryFlags::PRESENT) && !flags.contains(EntryFlags::HUGE_PAGE) {
            let tbl_addr = self as *const _ as usize;
            return Some((tbl_addr << 9) | (index << 12));
        }

        None
    }

    pub fn next_table(&self, index: usize) -> Option<&PageTable<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &*(addr as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut PageTable<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &mut *(addr as *mut _) })
    }

    pub fn next_table_create(
        &mut self,
        index: usize,
        allocator: &mut BitmapFrameAllocator,
    ) -> &mut PageTable<L::NextLevel> {
        // if the table doesn't exist, allocate it
        if self.next_table(index).is_none() {
            assert!(
                !self[index].flags().contains(EntryFlags::HUGE_PAGE),
                "mapping huge page as a table is not supported"
            );

            let frame = allocator.allocate_frame().expect("out of memory");
            self.entries[index].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap()
    }
}

impl<L: Level> Index<usize> for PageTable<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: Level> IndexMut<usize> for PageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}
