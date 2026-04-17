use core::alloc::Layout;

use alloc::collections::BTreeMap;

use crate::{kernel::USER_STACK_BOTTOM, utils};

pub struct VmSpace {
    /// key = start address
    map: BTreeMap<usize, Vma>,
}

#[derive(Debug, Clone, Copy)]
pub struct Vma {
    end: usize,
    ty: MappingType,
}

#[derive(Debug, Clone, Copy)]
pub enum MappingType {
    Anonymous,
}

impl VmSpace {
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, start: usize, end: usize, ty: MappingType) -> Result<(), ()> {
        assert!(start < end);

        // check previous vma for overlap
        if let Some((_, prev)) = self.map.range(..=start).next_back()
            && prev.end > start
        {
            return Err(()); // overlap
        }

        // check next vma for overlap
        if let Some((next, _)) = self.map.range(start..).next()
            && *next < end
        {
            return Err(()); // overlap
        }

        self.map.insert(start, Vma { end, ty });
        Ok(())
    }

    pub fn find_free_region(&self, layout: Layout) -> Option<usize> {
        // 1 GiB
        let mut current_addr = 0x4000_0000;

        for (&start, vma) in &self.map {
            let aligned_addr = utils::align_up(current_addr, layout.align());

            // Free memory before the current VMA
            if start >= aligned_addr && start - aligned_addr >= layout.size() {
                return Some(aligned_addr);
            }

            current_addr = current_addr.max(vma.end);
        }

        let aligned_addr = utils::align_up(current_addr, layout.align());
        let max_addr = USER_STACK_BOTTOM.0 as usize;

        if max_addr >= aligned_addr && max_addr - aligned_addr >= layout.size() {
            return Some(aligned_addr);
        } 

        log::error!("OOM: no suitable free region found for {:?}", layout);

        None
    }

    pub fn find(&self, addr: usize) -> Option<(usize, &Vma)> {
        let (start, vma) = self.map.range(..=addr).next_back()?;
        if addr < vma.end {
            return Some((*start, vma));
        }

        None
    }

    // TODO:
    // - unmap
    // - partial unmap
    // - merging adjacent VMAs
    // - interval trees??
}
