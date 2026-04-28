use core::arch::asm;

use crate::bus::port::Port;
#[cfg(test)]
use crate::kernel;
#[cfg(test)]
use spin::Once;

use crate::dbg_println;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        dbg_println!("test {}...", core::any::type_name::<T>());
        self();
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    ensure_test_kernel_init();
    dbg_println!("running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    dbg_println!("all tests passed");
    exit_qemu(QemuExitCode::Success)
}

pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    dbg_println!("[failed]");
    dbg_println!("{}", info);
    exit_qemu(QemuExitCode::Failed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(code: QemuExitCode) -> ! {
    unsafe {
        u32::write_to_port(0xF4, code as u32);

        loop {
            asm!("hlt")
        }
    }
}

#[cfg(test)]
static TEST_INIT: Once<()> = Once::new();

#[cfg(test)]
pub fn ensure_test_kernel_init() {
    TEST_INIT.call_once(kernel::init_for_tests);
}

#[cfg(not(test))]
pub fn ensure_test_kernel_init() {}
