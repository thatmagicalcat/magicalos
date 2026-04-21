use core::alloc::Layout;

use crate::{
    errno,
    memory::{self, MappingType, paging::PageTableEntryFlags},
    scheduler, utils,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MemMapFlags: i32 {
        const MAP_FILE    = 0x00;
        const MAP_SHARED  = 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_FIXED   = 0x10;
        const MAP_ANON    = 0x20;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MmapProt: i32 {
        const PROT_NONE  = 0x00;
        const PROT_READ  = 0x01;
        const PROT_WRITE = 0x02;
        const PROT_EXEC  = 0x04;
    }
}

#[unsafe(no_mangle)]
pub(crate) fn sys_mmap(
    addr: usize,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: usize,
) -> isize {
    log::debug!("Enter sys_mmap");

    let length = if length == 0 {
        return -errno::EINVAL as _;
    } else {
        utils::align_up(length, memory::PAGE_SIZE)
    };

    let prot = MmapProt::from_bits_truncate(prot);
    let flags = MemMapFlags::from_bits_truncate(flags);

    if !flags.contains(MemMapFlags::MAP_ANON) {
        log::error!("NOT IMPLEMENTED: non-anonymous mmap");
        return -errno::ENOSYS as _;
    }

    if flags.contains(MemMapFlags::MAP_FIXED) {
        if addr == 0 {
            log::error!("MAP_FIXED flag requires a non-null address");
            return -errno::EINVAL as _;
        }

        if !addr.is_multiple_of(memory::PAGE_SIZE) {
            log::error!("MAP_FIXED address must be page-aligned");
            return -errno::EINVAL as _;
        }
    }

    if flags.contains(MemMapFlags::MAP_SHARED) && flags.contains(MemMapFlags::MAP_PRIVATE) {
        log::error!("MAP_SHARED and MAP_PRIVATE flags are mutually exclusive");
        return -errno::EINVAL as _;
    }

    if !flags.contains(MemMapFlags::MAP_SHARED) && !flags.contains(MemMapFlags::MAP_PRIVATE) {
        log::error!("Either MAP_SHARED or MAP_PRIVATE flag must be set");
        return -errno::EINVAL as _;
    }

    scheduler::with_current_task(|task| {
        let addr = match get_mmap_addr(addr, length, flags, task) {
            Ok(v) => v,
            Err(v) => return v,
        };

        log::debug!(
            "sys_mmap(): addr={addr:#x}, length={length}, prot={prot:?}, flags={flags:?}, fd={fd}, offset={offset}",
        );

        let flags = get_page_table_entry_flags(prot);
        let start = utils::align_down(addr as _, memory::PAGE_SIZE);

        if let Err(e) = task
            .vmspace
            .insert(start, start + length, flags, MappingType::Anonymous)
        {
            log::error!("Failed to insert VMA entry: {e:?}");
            return -errno::EINVAL as _;
        }

        start as _
    })
}

fn get_page_table_entry_flags(prot: MmapProt) -> PageTableEntryFlags {
    let mut flags = PageTableEntryFlags::empty();

    if prot.contains(MmapProt::PROT_READ) {
        flags |= PageTableEntryFlags::USER_ACCESSIBLE;
    }

    if prot.contains(MmapProt::PROT_WRITE) {
        flags |= PageTableEntryFlags::USER_ACCESSIBLE | PageTableEntryFlags::WRITABLE;
    }

    if !prot.contains(MmapProt::PROT_EXEC) {
        flags |= PageTableEntryFlags::NO_EXECUTE;
    }

    flags
}

fn get_mmap_addr(
    hint: usize,
    length: usize,
    flags: MemMapFlags,
    task: &scheduler::Task,
) -> Result<usize, isize> {
    Ok(if flags.contains(MemMapFlags::MAP_FIXED) {
        // alignment check
        if !hint.is_multiple_of(memory::PAGE_SIZE) {
            log::error!("MAP_FIXED address must be page-aligned");
            return Err(-errno::EINVAL as _);
        }

        // overlap check
        if task.vmspace.find(hint).is_some() {
            log::error!("MAP_FIXED address cannot be inserted into memory mapping");
            return Err(-errno::EINVAL as _);
        }

        hint
    } else if hint == 0
        || !hint.is_multiple_of(memory::PAGE_SIZE)
        || task.vmspace.find(hint).is_some()
    {
        // if overlaps or not page-aligned, we need to find a new address
        let layout = Layout::from_size_align(length, memory::PAGE_SIZE).unwrap();
        let Some(new_addr) = task.vmspace.find_free_region(layout) else {
            log::error!("OOM: no suitable free region found for {:?}", layout);
            return Err(-errno::ENOMEM as _);
        };

        new_addr
    } else {
        // we can just use it
        hint
    })
}
