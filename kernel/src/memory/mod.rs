use core::ops::Range;

use multiboot2::MemoryAreaType;

use crate::kernel_bounds;
pub use bitmapframealloc::BitmapFrameAllocator;

mod bitmapframealloc;
