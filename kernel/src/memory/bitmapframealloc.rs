use core::ops::Range;

use multiboot2::MemoryAreaType;

use crate::{
    kernel_bounds,
    memory::FrameAllocator,
};

use super::{Frame, PAGE_SIZE};

const USED: u8 = !FREE;
const FREE: u8 = 0;

pub struct BitmapFrameAllocator {
    bitmap_slice: &'static mut [u8],
    total_frames: usize,
    last_allocated_frame: usize,
}

impl BitmapFrameAllocator {
    pub fn new(boot_info: &multiboot2::BootInformation) -> Self {
        let (_, kernel_end) = kernel_bounds();
        let memory_areas = boot_info
            .memory_map_tag()
            .expect("Memory map tag not found")
            .memory_areas();
        let highest_address = memory_areas
            .iter()
            .map(|area| area.end_address())
            .max()
            .expect("No memory areas found") as usize;
        let total_frames = highest_address / PAGE_SIZE;
        let bitmap_array_size = total_frames / 8;
        let align_mask = PAGE_SIZE - 1;
        let bitmap_array_start_ptr = if kernel_end & align_mask == 0 {
            // already aligned
            kernel_end
        } else {
            (kernel_end | align_mask) + 1
        } as *mut u8;

        let bitmap_slice =
            unsafe { core::slice::from_raw_parts_mut(bitmap_array_start_ptr, bitmap_array_size) };

        bitmap_slice.fill(USED);

        let mut allocator = Self {
            total_frames,
            bitmap_slice,
            last_allocated_frame: 0,
        };

        // mark the available memory areas as free
        memory_areas
            .iter()
            .filter(|area| area.typ() == MemoryAreaType::Available)
            .for_each(|area| {
                let start_frame = area.start_address() as usize / PAGE_SIZE;
                let end_frame = (area.end_address() as usize).div_ceil(PAGE_SIZE);
                allocator.mark_frames_free(start_frame..end_frame);
            });

        // mark the memory used by kernel as used
        let kernel_start_frame = kernel_bounds().0 / PAGE_SIZE;
        let kernel_end_frame = kernel_bounds().1.div_ceil(PAGE_SIZE);
        allocator.mark_frames_used(kernel_start_frame..kernel_end_frame);

        // mark the multiboot info structure as used
        let mb_start_frame = boot_info.start_address() / PAGE_SIZE;
        let mb_end_frame = boot_info.end_address().div_ceil(PAGE_SIZE);
        allocator.mark_frames_used(mb_start_frame..mb_end_frame);

        // mark the bitmap array itself as used
        let bitmap_start_frame = bitmap_array_start_ptr as usize / PAGE_SIZE;
        let bitmap_end_frame =
            (bitmap_array_start_ptr as usize + bitmap_array_size).div_ceil(PAGE_SIZE);
        allocator.mark_frames_used(bitmap_start_frame..bitmap_end_frame);

        // mark the first frame as used to avoid allocating the null pointer
        allocator.mark_frame_used(0);

        allocator
    }

    fn allocate_frame_helper(&mut self, offset: usize) -> Option<Frame> {
        self.bitmap_slice
            .iter()
            .enumerate()
            .skip(offset)
            .filter(|(_, byte)| **byte != !0)
            .map(|(byte_idx, byte)| Frame(byte_idx * 8 + byte.trailing_ones() as usize))
            .next()
            .inspect(|&Frame(frame_idx)| {
                self.last_allocated_frame = frame_idx + 1;
                self.mark_frame_used(frame_idx)
            })
    }

    #[inline(always)]
    pub fn is_frame_free(&self, frame_index: usize) -> bool {
        let byte_index = frame_index >> 3;
        let bit_index = frame_index & 7;
        (self.bitmap_slice[byte_index] & (1 << bit_index)) == FREE
    }

    #[inline(always)]
    pub fn is_frame_used(&self, frame_index: usize) -> bool {
        !self.is_frame_free(frame_index)
    }

    #[inline(always)]
    pub fn mark_frames_used(&mut self, range: Range<usize>) {
        for frame_index in range {
            self.mark_frame_used(frame_index);
        }
    }

    #[inline(always)]
    pub fn mark_frames_free(&mut self, range: Range<usize>) {
        for frame_index in range {
            self.mark_frame_free(frame_index);
        }
    }

    #[inline(always)]
    pub fn mark_frame_used(&mut self, frame_index: usize) {
        let byte_index = frame_index >> 3;
        let bit_index = frame_index & 7;
        self.bitmap_slice[byte_index] |= 1 << bit_index;
    }

    #[inline(always)]
    pub fn mark_frame_free(&mut self, frame_index: usize) {
        let byte_index = frame_index >> 3;
        let bit_index = frame_index & 7;
        self.bitmap_slice[byte_index] &= !(1 << bit_index);
    }

    pub const fn total_frames(&self) -> usize {
        self.total_frames
    }
}

impl FrameAllocator for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        self.allocate_frame_helper(self.last_allocated_frame >> 3)
            .or_else(|| self.allocate_frame_helper(0))
    }

    fn deallocate_frame(&mut self, Frame(frame_index): Frame) {
        if frame_index >= self.total_frames {
            panic!("Frame index out of bounds: {}", frame_index);
        }

        self.mark_frame_free(frame_index);
    }

    fn bounds(&self) -> (usize, usize) {
        let start = self.bitmap_slice.as_ptr() as usize;
        let end = start + self.bitmap_slice.len();
        (start, end)
    }
}
