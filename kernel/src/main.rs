#![no_std]
#![no_main]

extern crate alloc;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    magicalos_kernel::kernel_entry();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    magicalos_kernel::panic(info)
}
