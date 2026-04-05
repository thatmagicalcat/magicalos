use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    ptr,
};

use limine::*;

#[repr(transparent)]
pub struct Wrapper<T>(UnsafeCell<T>);

// SAFETY: We are in a single-threaded pre-boot environment, and the bootloader
// only modifies this memory before the kernel entry point.
unsafe impl<T> Sync for Wrapper<T> {}

impl<T> Deref for Wrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Read directly from the UnsafeCell
        unsafe { &*self.0.get() }
    }
}

impl<T> DerefMut for Wrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.get() }
    }
}

macro_rules! with_common_magic {
    [ $($e:expr),* $(,)? ] => {
        [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, $($e),*]
    };
}

macro_rules! limine_requests {
    [
        $(
            $(#[$meta:meta])*
            $vis:vis static $name:ident: $ty:ty = $e:expr;
        )*
    ] => {
        $(
            $(#[$meta])*
            #[used]
            #[unsafe(link_section = ".limine_requests")]
            $vis static $name: Wrapper<$ty> = Wrapper(UnsafeCell::new($e));
        )*
    };
}

const LIMINE_HHDM_REQUEST_ID: [u64; 4] = with_common_magic![0x48dcf1cb8ad2b852, 0x63984e959a98244b];
const LIMINE_RSDP_REQUEST_ID: [u64; 4] = with_common_magic![0xc5e77b6b397e7b43, 0x27637845accdcf3c];
const LIMINE_MMAP_REQUEST_ID: [u64; 4] = with_common_magic![0x67cf3d9d378a806f, 0xe304acdfc50c3c62];
const LIMINE_FB_REQUEST_ID: [u64; 4] = with_common_magic![0x9d5827dcd881dd75, 0xa3148604f6fab11b];
const LIMINE_FLANTERM_FB_INIT_PARAMS_REQUEST_ID: [u64; 4] =
    with_common_magic![0x3259399fe7c5f126, 0xe01c1c8c5db9d1a9];

#[used]
#[unsafe(link_section = ".limine_requests_start")]
pub static LIMINE_REQUESTS_START: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static BASE_REVISION: [u64; 3] = [0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 6];

limine_requests! {
    pub static FRAMEBUFFER_REQUEST: limine_framebuffer_request = limine_framebuffer_request {
        id: LIMINE_FB_REQUEST_ID,
        revision: 0,
        response: ptr::null_mut(),
    };

    pub static HHDM_REQUEST: limine_hhdm_request = limine_hhdm_request {
        id: LIMINE_HHDM_REQUEST_ID,
        revision: 0,
        response: ptr::null_mut(),
    };

    pub static MEMMAP: limine_memmap_request = limine_memmap_request {
        id: LIMINE_MMAP_REQUEST_ID,
        revision: 0,
        response: ptr::null_mut(),
    };

    pub static RSDP_REQUEST: limine_rsdp_request = limine_rsdp_request {
        id: LIMINE_RSDP_REQUEST_ID,
        revision: 0,
        response: ptr::null_mut(),
    };

    pub static FLANTERM_FB_INIT_PARAMS_REQUEST: limine_flanterm_fb_init_params_request =
        limine_flanterm_fb_init_params_request {
            id: LIMINE_FLANTERM_FB_INIT_PARAMS_REQUEST_ID,
            revision: 0,
            response: ptr::null_mut(),
        };
}

#[used]
#[unsafe(link_section = ".limine_requests_end")]
pub static LIMINE_REQUESTS_END: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];
