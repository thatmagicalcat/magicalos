use alloc::collections::BTreeMap;

pub struct VmSpace {
    /// key = start address
    map: BTreeMap<usize, Vma>,
}

#[derive(Debug, Clone, Copy)]
pub struct Vma {
    end: usize,
    flags: Flags,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Flags: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

impl VmSpace {
    pub const fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, start: usize, end: usize, flags: Flags) -> Result<(), ()> {
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

        self.map.insert(start, Vma { end, flags });
        Ok(())
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
