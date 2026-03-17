// use core::array::from_fn;
//
// use crate::memory::{Frame, FrameAllocator};
//
// pub struct TinyAllocator<const N: usize> {
//     frames: [Option<Frame>; N],
// }
//
// impl<const N: usize> TinyAllocator<N> {
//     pub fn new<A: FrameAllocator>(allocator: &mut A) -> Self {
//         Self {
//             frames: from_fn(|_| allocator.allocate_frame()),
//         }
//     }
//
//     pub const fn empty() -> Self {
//         Self { frames: [None; N] }
//     }
//
//     pub fn destroy<A: FrameAllocator>(self, allocator: &mut A) {
//         for frame in self.frames.into_iter().flatten() {
//             allocator.deallocate_frame(frame);
//         }
//     }
// }
//
// impl<const N: usize> FrameAllocator for TinyAllocator<N> {
//     fn allocate_frame(&mut self) -> Option<Frame> {
//         self.frames.iter_mut().find_map(Option::take)
//     }
//
//     fn deallocate_frame(&mut self, frame: Frame) {
//         self.frames
//             .iter_mut()
//             .find(|slot| slot.is_none())
//             .expect("TinyAllocator is full, cannot deallocate frame")
//             .replace(frame);
//     }
//
//     fn bounds(&self) -> (usize, usize) {
//
//     }
// }
