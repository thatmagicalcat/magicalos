use core::alloc::Layout;

use crate::{errno, memory, scheduler, utils};

bitflags::bitflags! {
    pub struct MmapFlags: i32 {
        const MAP_FILE      = 0x00;
        const MAP_SHARED    = 0x01;
        const MAP_PRIVATE   = 0x02;
        const MAP_FIXED     = 0x10;
        const MAP_ANON      = 0x20;
        const MAP_ANONYMOUS = 0x20;
    }

    pub struct MmapProt: i32 {
        const PROT_NONE  = 0x00;
        const PROT_READ  = 0x01;
        const PROT_WRITE = 0x02;
        const PROT_EXEC  = 0x04;
    }
}

#[unsafe(no_mangle)]
pub(crate) fn sys_mmap(
    addr: *mut u8,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: usize,
) -> isize {
    log::debug!("Enter sys_mmap");

    // let addr = if !addr.is_null() {
    //     addr
    // } else {
    //     unsafe {
    //         scheduler::allocate_vmm(Layout::from_size_align(length, 4096).unwrap())
    //             .expect("Failed to allocate memory for mmap")
    //             .as_mut()
    //     }
    // };
    //
    // let length = if length == 0 {
    //     return -errno::EINVAL as _;
    // } else {
    //     // align to page size
    //     utils::align_up(length, memory::PAGE_SIZE)
    // };
    //
    // let prot = MmapProt::from_bits_truncate(prot);
    // let flags = MmapFlags::from_bits_truncate(flags);
    //
    // if !flags.contains(MmapFlags::MAP_ANON) {
    //     log::error!("NOT IMPLEMENTED: non-anonymous mmap");
    //     return -errno::ENOIMPL as _;
    // }

    todo!()
}
