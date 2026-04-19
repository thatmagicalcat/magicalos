#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    kernel::kernel_entry()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::panic(info)
}
