#![no_std]
#![no_main]
#![allow(unused)]

use core::fmt::Write;

use crate::vga_buffer::{Buffer, Color, Writer};

mod vga_buffer;
mod volatile;

const VGA_MEMORY_ADDRESS: usize = 0xB8000;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let buffer = unsafe { &mut *(VGA_MEMORY_ADDRESS as *mut Buffer) };

    let mut writer = Writer::new(buffer);
    writer.initialize();
    writer.set_color(Color::LightGreen, Color::Black);

    _ = write!(writer, "hello, world!");

    #[allow(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
