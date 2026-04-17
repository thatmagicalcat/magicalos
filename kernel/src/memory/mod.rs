#![allow(static_mut_refs)]

pub use bitmapframealloc::BitmapFrameAllocator;
use spin::Mutex;

mod bitmapframealloc;
pub mod heap;
pub mod paging;
mod tinyalloc;
pub mod vmm;
mod vmspace;

pub use vmm::{KERNEL_VMM, init_vmm};
pub use vmspace::*;

pub const PAGE_SIZE: usize = 1024 * 4;

pub static mut GLOBAL_FRAME_ALLOCATOR: Option<Mutex<BitmapFrameAllocator>> = None;

pub fn init_global_frame_allocator(memmap: &[*mut limine::limine_memmap_entry]) {
    unsafe {
        GLOBAL_FRAME_ALLOCATOR = Some(Mutex::new(BitmapFrameAllocator::new(memmap)));
    }
}

pub fn lock_global_frame_allocator<'a>() -> spin::MutexGuard<'a, BitmapFrameAllocator> {
    unsafe { GLOBAL_FRAME_ALLOCATOR.as_mut().unwrap().lock() }
}

pub fn allocate_frame() -> Option<Frame> {
    unsafe {
        GLOBAL_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .lock()
            .allocate_frame()
    }
}

pub fn deallocate_frame(frame: Frame) {
    unsafe {
        GLOBAL_FRAME_ALLOCATOR
            .as_mut()
            .unwrap()
            .lock()
            .deallocate_frame(frame)
    };
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Physical Frame
pub struct Frame(pub usize);

impl Frame {
    pub const fn get_ptr(&self) -> *mut u8 {
        (self.0 * PAGE_SIZE) as *mut u8
    }

    pub const fn start_address(&self) -> usize {
        self.0 * PAGE_SIZE
    }

    pub const fn end_address(&self) -> usize {
        (self.0 + 1) * PAGE_SIZE
    }

    pub const fn from_addr(addr: usize) -> Self {
        Self(addr / PAGE_SIZE)
    }
}
