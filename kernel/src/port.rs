// source: https://bob.cs.sonoma.edu/IntroCompOrg-x64/bookch16.html#:~:text=11,init%5Fio

use core::arch::asm;

/// Wait for a small amount of time (1 to 4 microseconds generally)
pub fn io_wait() {
    unsafe { u8::write_to_port(0x80, 0) };
}

pub trait Port {
    unsafe fn read_from_port(port: u16) -> Self;
    unsafe fn write_to_port(port: u16, value: Self);
}

impl Port for u8 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> Self {
        let value: Self;
        unsafe {
            asm!(
                "in al, dx",
                out("al") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        };

        value
    }

    #[inline]
    unsafe fn write_to_port(port: u16, value: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("al") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        }
    }
}

impl Port for u16 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> Self {
        let value: Self;
        unsafe {
            asm!(
                "in ax, dx",
                out("ax") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        };

        value
    }

    #[inline]
    unsafe fn write_to_port(port: u16, value: Self) {
        unsafe {
            asm!(
                "out dx, ax",
                in("ax") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        }
    }
}

impl Port for u32 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> Self {
        let value: Self;
        unsafe {
            asm!(
                "in eax, dx",
                out("eax") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        };

        value
    }

    #[inline]
    unsafe fn write_to_port(port: u16, value: Self) {
        unsafe {
            asm!(
                "out dx, eax",
                in("eax") value,
                in("dx") port,
                options(nomem, nostack, preserves_flags)
            )
        }
    }
}
