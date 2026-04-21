#![no_std]
#![no_main]

extern crate alloc;

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    magicalos_kernel::kentry();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    magicalos_kernel::panic(info)
}
