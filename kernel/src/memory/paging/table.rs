// The 511th entry of the P4 table is mapped to the P4 table itself

use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::memory::{
    Frame, PAGE_SIZE,
    paging::{EntryFlags, PageTableEntry},
};

use super::{ENTRIES_PER_TABLE, PhysAddr, VirtAddr};

pub const P4: *mut Table<L4> = 0xFFFFFFFFFFFFF000 as *mut _;

pub trait TableLevel {}
pub trait HierarchicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

pub enum L4 {}
pub enum L3 {}
pub enum L2 {}
pub enum L1 {}

impl TableLevel for L4 {}
impl TableLevel for L3 {}
impl TableLevel for L2 {}
impl TableLevel for L1 {}

impl HierarchicalLevel for L4 {
    type NextLevel = L3;
}

impl HierarchicalLevel for L3 {
    type NextLevel = L2;
}

impl HierarchicalLevel for L2 {
    type NextLevel = L1;
}

pub struct Table<L> {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
    _phantom: PhantomData<L>,
}

impl<L: TableLevel> Table<L> {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused()
        }
    }
}

impl<L: HierarchicalLevel> Table<L> {
    /// formula:
    /// next_table_addr = table_addr << 9 | index << 12
    fn next_table_addr(&self, index: usize) -> Option<usize> {
        let flags = self[index].flags();
        if flags.contains(super::EntryFlags::PRESENT) && !flags.contains(EntryFlags::HUGE_PAGE) {
            let tbl_addr = self as *const _ as usize;
            return Some((tbl_addr << 9) | (index << 12));
        }

        None
    }

    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &*(addr as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &mut *(addr as *mut _) })
    }
}

impl<L> Index<usize> for Table<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub fn translate(virt_addr: VirtAddr) -> Option<PhysAddr> {
    let offset = virt_addr.0 % PAGE_SIZE;
    // translate_page(virt_addr.containing_frame());
    todo!()
}

fn translate_page(page: VirtAddr) -> Option<Frame> {
    let p3 = unsafe { &*P4 }.next_table(page.p4_idx());
    let huge_page = || todo!();

    p3.and_then(|p3| p3.next_table(page.p3_idx()))
        .and_then(|p2| p2.next_table(page.p2_idx()))
        .and_then(|p1| p1[page.p1_idx()].pointed_frame())
        .or_else(huge_page)
}
