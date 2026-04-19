use crate::bus::port::Port;
#[cfg(test)]
use crate::kernel;
#[cfg(test)]
use spin::Once;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(code: QemuExitCode) -> ! {
    unsafe {
        u32::write_to_port(0xF4, code as u32);
    }

    crate::halt_loop()
}

#[cfg(test)]
static TEST_INIT: Once<()> = Once::new();

#[cfg(test)]
pub fn ensure_test_kernel_init() {
    TEST_INIT.call_once(kernel::init_for_tests);
}

#[cfg(not(test))]
pub fn ensure_test_kernel_init() {}
