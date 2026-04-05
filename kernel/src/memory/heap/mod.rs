use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use spin::{Mutex, MutexGuard};

use crate::{
    memory::{Frame, FrameAllocator, PAGE_SIZE, paging::{EntryFlags, Mapper, VirtualAddress}},
};

mod linkedlist_alloc;
pub use linkedlist_alloc::LinkedListAllocator;

// 100 MiB
pub const HEAP_SIZE: usize = 100 * 1024 * 1024;
pub const HEAP_START: usize = 0x40_00_00_00;

#[global_allocator]
pub static GLOBAL_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

/// Map the heap memory
pub fn init<A: FrameAllocator>(mapper: &mut Mapper, allocator: &mut A) {
    log::info!("Initializing Kernel Heap Memory");

    log::info!(
        "Mapping heap memory from {:#010x} to {:#010x}, size: {} KiB",
        HEAP_START,
        HEAP_START + HEAP_SIZE,
        HEAP_SIZE / 1024,
    );

    // remap the memory used by the heap allocator
    let heap_mem_start = Frame::from_addr(HEAP_START);
    let heap_mem_end = Frame((HEAP_SIZE + HEAP_START).div_ceil(PAGE_SIZE));

    for frame in heap_mem_start.0..heap_mem_end.0 {
        let page = VirtualAddress((frame * PAGE_SIZE) as _);
        mapper.map(page, EntryFlags::PRESENT | EntryFlags::WRITABLE, allocator);
    }

    log::info!("Initializing global allocator");

    // initialize the heap
    GLOBAL_ALLOCATOR
        .lock()
        .init(HEAP_START as *mut _, HEAP_SIZE);
}

pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, A> {
        self.inner.lock()
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .kmalloc(layout)
            .map(|i| i.as_ptr())
            .unwrap_or_default()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock()
            .kfree(unsafe { NonNull::new_unchecked(ptr) }, layout);
    }
}
