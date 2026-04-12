use core::{alloc::Layout, ptr::NonNull};

use crate::utils::align_up;

struct FreeBlock {
    size: usize,
    next: Option<NonNull<Self>>,
}

impl FreeBlock {
    const MIN_SIZE: usize = core::mem::size_of::<Self>();
    const ALIGNMENT: usize = core::mem::align_of::<Self>();

    fn start_addr(&self) -> usize {
        &raw const *self as _
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    heap_size: usize,
    first: FreeBlock,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            heap_size: 0,
            first: FreeBlock {
                size: 0,
                next: None,
            },
        }
    }

    pub fn init(&mut self, heap_start: *mut u8, heap_size: usize) {
        log::info!(
            "Initializing LinkedListAllocator with heap start at {:#010x} and size {} KiB",
            heap_start as usize,
            heap_size / 1024
        );

        assert!(heap_size >= FreeBlock::MIN_SIZE);

        unsafe {
            core::ptr::write(
                heap_start as *mut _,
                FreeBlock {
                    size: heap_size,
                    next: None,
                },
            );
        }

        self.heap_size = heap_size;
        self.first = FreeBlock {
            size: 0,
            next: Some(NonNull::new(heap_start as _).unwrap()),
        };
    }

    /// Min size and alignment is always equal to FreeBlock's size and alignment
    pub fn kmalloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let effective_size = layout.size().max(FreeBlock::MIN_SIZE);
        let effective_align = layout.align().max(FreeBlock::ALIGNMENT);

        let mut last = &mut self.first;

        while let Some(mut current_ptr) = last.next {
            let current = unsafe { current_ptr.as_mut() };

            let mut alloc_start = align_up(current.start_addr(), effective_align);
            let mut front_pad = alloc_start - current.start_addr();

            if front_pad > 0 && front_pad < FreeBlock::MIN_SIZE {
                alloc_start = align_up(current.start_addr() + FreeBlock::MIN_SIZE, effective_align);
                front_pad = alloc_start - current.start_addr();
            }

            let alloc_end = alloc_start + effective_size;

            if alloc_end <= current.end_addr() {
                let rem_start = align_up(alloc_end, FreeBlock::ALIGNMENT);
                let rem_size = current.end_addr().saturating_sub(rem_start);

                let mut next_free = current.next.take();

                if rem_size >= FreeBlock::MIN_SIZE {
                    let new_block_ptr = rem_start as *mut FreeBlock;
                    let new_block = FreeBlock {
                        size: rem_size,
                        next: next_free,
                    };
                    unsafe { core::ptr::write(new_block_ptr, new_block) };
                    next_free = NonNull::new(new_block_ptr);
                }

                if front_pad >= FreeBlock::MIN_SIZE {
                    current.size = front_pad;
                    current.next = next_free;
                } else {
                    last.next = next_free;
                }

                // if log::log_enabled!(log::Level::Trace) {
                //     let es = format_args!(" (effective: {})", effective_size);
                //     let ea = format_args!(" (effective: {})", effective_align);
                //     let empty = format_args!("");
                //
                //     log::trace!(
                //         "kmalloc(): size = {}{}, align = {}{}",
                //         layout.size(),
                //         if layout.size() < FreeBlock::MIN_SIZE {
                //             es
                //         } else {
                //             empty
                //         },
                //         layout.align(),
                //         if layout.align() < FreeBlock::ALIGNMENT {
                //             ea
                //         } else {
                //             empty
                //         },
                //     );
                // }

                return NonNull::new(alloc_start as *mut _);
            }

            last = current;
        }

        log::error!("Heap: Out of memory, allocation of {effective_size} (align: {effective_align}) failed");
        None
    }

    pub fn kfree(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let effective_size = layout.size().max(FreeBlock::MIN_SIZE);
        let addr = ptr.as_ptr() as usize;

        // if log::log_enabled!(log::Level::Trace) {
        //     let effective_align = layout.align().max(FreeBlock::ALIGNMENT);
        //     let es = format_args!(" (effective: {effective_size})");
        //     let ea = format_args!(" (effective: {effective_align})");
        //     let empty = format_args!("");
        //
        //     log::trace!(
        //         "kfree(): size = {}{}, align = {}{}",
        //         layout.size(),
        //         if layout.size() < FreeBlock::MIN_SIZE {
        //             es
        //         } else {
        //             empty
        //         },
        //         layout.align(),
        //         if layout.align() < FreeBlock::ALIGNMENT {
        //             ea
        //         } else {
        //             empty
        //         },
        //     );
        // }

        let new_block_ptr = addr as *mut FreeBlock;
        let new_block = FreeBlock {
            size: effective_size,
            next: self.first.next.take(),
        };

        unsafe { core::ptr::write(new_block_ptr, new_block) };
        self.first.next = NonNull::new(new_block_ptr);
    }
}

// SAFETY: trust me bro
unsafe impl Send for LinkedListAllocator {}
unsafe impl Sync for LinkedListAllocator {}
