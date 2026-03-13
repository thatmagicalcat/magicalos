#![no_std]
#![no_main]
#![allow(unused)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]

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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    vga_buffer::WRITER
        .lock()
        .set_color(Color::LightRed, Color::Black);
    println!("Panic: {}", info);

    loop {}
}
