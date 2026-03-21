use spin::{Lazy, Mutex};

use crate::port::Port;

pub static QEMU_DEBUGCON: Lazy<Mutex<QemuDebugcon>> = Lazy::new(|| Mutex::new(QemuDebugcon));

pub struct QemuDebugcon;

impl QemuDebugcon {
    pub fn write_byte(&self, byte: u8) {
        unsafe {
            u8::write_to_port(0xE9, byte);
        }
    }
}

impl core::fmt::Write for QemuDebugcon {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }

        Ok(())
    }
}
