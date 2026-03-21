use core::fmt;

use spin::{Lazy, Mutex};

use crate::port::Port;

pub static SERIAL: Lazy<Mutex<SerialPort>> = Lazy::new(|| Mutex::new(SerialPort::new(0x3F8)));

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    pub const fn new(base: u16) -> Self {
        Self { base }
    }

    #[inline(always)]
    fn wb(&self, offset: u16, value: u8) {
        unsafe { u8::write_to_port(self.base + offset, value) };
    }

    #[inline(always)]
    fn rb(&self, offset: u16) -> u8 {
        unsafe { u8::read_from_port(self.base + offset) }
    }

    /// Initializes the UART chip to 115200 8N1.
    pub fn init(&self) {
        self.wb(1, 0x00); // Disable interrupts
        self.wb(3, 0x80); // Enable DLAB (set baud rate mask)
        self.wb(0, 0x01); // Set divisor to 1 (115200 baud)
        self.wb(1, 0x00); // (High byte of divisor)
        self.wb(3, 0x03); // 8 bits, no parity, one stop bit
        self.wb(2, 0xC7); // Enable FIFO, clear them, 14-byte threshold
        self.wb(4, 0x0B); // IRQs enabled, RTS/DSR set
    }

    /// Sends a single byte, polling the line status until ready.
    pub fn send(&self, byte: u8) {
        while self.rb(5) & 0x20 == 0 {
            core::hint::spin_loop();
        }

        self.wb(0, byte);
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.send(b'\r');
            }
            self.send(byte);
        }

        Ok(())
    }
}
