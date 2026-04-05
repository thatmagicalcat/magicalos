use core::ops::Range;

use crate::{limine_requests::HHDM_REQUEST, memory::FrameAllocator};

use super::{Frame, PAGE_SIZE};

const USED: u8 = !FREE;
const FREE: u8 = 0;

#[derive(Debug)]
pub struct BitmapFrameAllocator {
    bitmap_slice: &'static mut [u8],
    total_frames: usize,
    usable_frames: usize,
    allocated_frames: usize,
    last_allocated_frame: usize,
}

impl BitmapFrameAllocator {
    pub fn new(entries: &[*mut limine::limine_memmap_entry]) -> Self {
        let hhdm_offset = unsafe { (*HHDM_REQUEST.response).offset as usize };
        let highest_address = entries
            .iter()
            .map(|&entry_ptr| unsafe { &*entry_ptr })
            .map(|entry| entry.base + entry.length)
            .max()
            .expect("No memory areas found") as usize;

        let total_frames = highest_address / PAGE_SIZE;
        let bitmap_array_size = total_frames / 8;

        // find a block of memory which is big enough to hold the bitmap slice
        let blocks = entries
            .iter()
            .map(|&entry_ptr| unsafe { &*entry_ptr })
            .find(|entry| entry.length as usize >= bitmap_array_size)
            .expect("Could not find a memory area large enough to hold the bitmap slice");

        let bitmap_array_start_virt_addr = blocks.base as usize + hhdm_offset;

        let bitmap_slice = unsafe {
            core::slice::from_raw_parts_mut(
                bitmap_array_start_virt_addr as *mut u8,
                bitmap_array_size,
            )
        };

        bitmap_slice.fill(USED);

        log::info!(
            "Bitmap frame allocator initialized with total frames: {}, bitmap size: {} KiB, bitmap start: {:#X}",
            total_frames,
            bitmap_array_size / 1024,
            bitmap_array_start_virt_addr
        );

        let mut allocator = Self {
            total_frames,
            bitmap_slice,
            allocated_frames: 0,
            last_allocated_frame: 0,
            usable_frames: 0,
        };

        // mark the usable memory areas as free
        entries
            .iter()
            .map(|&entry_ptr| unsafe { &*entry_ptr })
            .filter(|entry| entry.type_ == limine::LIMINE_MEMMAP_USABLE as u64)
            .for_each(|entry| {
                let start_frame = entry.base as usize / PAGE_SIZE;
                let end_frame = ((entry.base + entry.length) as usize).div_ceil(PAGE_SIZE);
                allocator.usable_frames += end_frame - start_frame;
                allocator.mark_frames_free(start_frame..end_frame);
            });

        // everything else is already marked as used, so we don't need to do anything for reserved
        // areas

        // mark the bitmap array itself as used
        let bitmap_phys_base = blocks.base as usize;
        let bitmap_start_frame = bitmap_phys_base / PAGE_SIZE;
        let bitmap_end_frame = (bitmap_phys_base + bitmap_array_size).div_ceil(PAGE_SIZE);

        allocator.mark_frames_used(bitmap_start_frame..bitmap_end_frame);

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
        self.allocated_frames += 1;

        // if log::log_enabled!(log::Level::Debug) {
        //     let allocated_size_kb = self.allocated_frames * PAGE_SIZE / 1024;
        //     let total_size_kb = self.total_frames * PAGE_SIZE / 1024;
        //
        //     log::trace!(
        //         "allocate_frame(): [{}/{}] ({} KiB used / {} KiB free), last allocated frame: {}",
        //         self.allocated_frames,
        //         self.total_frames,
        //         allocated_size_kb,
        //         total_size_kb,
        //         self.last_allocated_frame
        //     );
        // }

        self.allocate_frame_helper(self.last_allocated_frame >> 3)
            .or_else(|| self.allocate_frame_helper(0))
    }

    fn deallocate_frame(&mut self, Frame(frame_index): Frame) {
        // log::trace!("deallocate_frame({})", frame_index);

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
