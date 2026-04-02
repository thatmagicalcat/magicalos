#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]
#![allow(clippy::empty_loop)]

#[rustfmt::skip]
const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

/// The virtual address where the Linear framebuffer is mapped
const LFB_VIRT_ADDR: usize = 0xFFFF_8000_0000_0000;

const WALLPAPER_DATA: &[u8] = include_bytes!("../../wallpaper.bin");
const FONT_DATA: &[u8] = include_bytes!("../../ter-u32n.psf");

unsafe extern "C" {
    static kernel_start: [u8; 0];
    static kernel_end: [u8; 0];
}

extern crate alloc;

mod gdt;
mod graphics;
mod hpet;
mod interrupts;
mod io;
mod ioapic;
mod kernel;
mod macros;
mod memory;
mod scheduler;
mod task;
mod thread;
mod utils;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    kernel::init(multiboot_info_addr);

    let mut locked = graphics::get_window_console().lock();

    // load the wallpaper
    for y in 0..locked.info.height {
        for x in 0..locked.info.width {
            let idx = (y * locked.info.width + x) as usize;
            let pixel_data = &WALLPAPER_DATA[idx * 4..idx * 4 + 4];
            let color = u32::from_le_bytes(pixel_data.try_into().unwrap());

            let r = ((color >> locked.info.r_shift) & 0xFF) as u8;
            let g = ((color >> locked.info.g_shift) & 0xFF) as u8;
            let b = ((color >> locked.info.b_shift) & 0xFF) as u8;

            locked.write_pixel(x, y, r, g, b);
        }
    }

    for c in "Hello, World!".chars() {
        locked.write_char(c);
    }

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

pub struct KernelBounds {
    pub start: usize,
    pub end: usize,
}

/// Returns the start and end addresses of the kernel in memory.
pub fn kernel_bounds() -> KernelBounds {
    unsafe {
        KernelBounds {
            start: kernel_start.as_ptr() as usize,
            end: kernel_end.as_ptr() as usize,
        }
    }
}
