#![no_std]
#![no_main]
#![allow(unused)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]

// SAFETY: trust me bro
unsafe extern "C" {
    safe static kernel_start: [u8; 0];
    safe static kernel_end: [u8; 0];
}

use core::{arch::asm, fmt::Write, ptr};

use vga_buffer::{Buffer, Color, Writer};

mod interrupts;
mod macros;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    println!("Hello, World!");
    interrupts::init();

    unsafe { *(0xDEADBEEF as *mut u32) = 42 };

    #[allow(clippy::empty_loop)]
    loop {}
}

/// Returns the start and end addresses of the kernel in memory.
pub fn kernel_bounds() -> (usize, usize) {
    unsafe {
        (
            &kernel_start as *const _ as usize,
            &kernel_end as *const _ as usize,
        )
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    vga_buffer::WRITER
        .lock()
        .set_color(Color::LightRed, Color::Black);
    println!("Panic: {}", info);

    loop {}
}
