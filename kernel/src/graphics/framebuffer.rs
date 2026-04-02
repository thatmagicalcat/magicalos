use lazy_static::lazy_static;
use spin::{Mutex, Once};

use crate::{graphics::psf2::PSF2Font, LFB_VIRT_ADDR};

lazy_static! {
    pub static ref WINDOW_CONSOLE: Once<Mutex<WindowConsole>> = Once::new();
}

pub fn init_window_console(info: FrameBufferInfo, font: PSF2Font) {
    assert!(
        info.bits_per_pixel == 32,
        "Only 32bpp framebuffers are supported... this will cause bleeding"
    );

    let console = WindowConsole {
        info,
        buffer: FrameBuffer {
            ptr: LFB_VIRT_ADDR as *mut _,
        },
        font,
        x: 0,
        y: 0,
        foreground: 0xFFFFFF,
        background: 0x000000,
    };

    WINDOW_CONSOLE.call_once(|| Mutex::new(console));
}

pub fn get_window_console() -> &'static Mutex<WindowConsole> {
    WINDOW_CONSOLE
        .get()
        .expect("Window console not initialized")
}

pub struct WindowConsole {
    pub info: FrameBufferInfo,
    pub buffer: FrameBuffer,
    pub font: PSF2Font,
    pub x: u32,
    pub y: u32,
    pub foreground: u32,
    pub background: u32,
}

impl WindowConsole {
    pub const fn change_colors(&mut self, foreground: u32, background: u32) {
        self.foreground = foreground;
        self.background = background;
    }

    pub fn write_char(&mut self, c: char) {
        self.font.write_char(
            c,
            self.x,
            self.y,
            self.foreground,
            None,
            self,
        );

        self.x += self.font.width;
        if self.x >= self.info.width {
            self.x = 0;
            self.y += self.font.height;
        }
    }

    pub fn write_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let FrameBufferInfo {
            width,
            height,
            bits_per_pixel,
            pitch,
            r_shift,
            g_shift,
            b_shift,
        } = self.info;

        if x >= width || y >= height {
            return;
        }

        let bytes_per_pixel = bits_per_pixel as u32 / 8;
        let byte_offset = (y * pitch) + (x * bytes_per_pixel);

        unsafe {
            let pixel_ptr = self.buffer.ptr.add(byte_offset as usize);

            if bits_per_pixel == 32 {
                let color =
                    ((r as u32) << r_shift) | ((g as u32) << g_shift) | ((b as u32) << b_shift);

                (pixel_ptr as *mut u32).write_unaligned(color);
            } else {
                // if we write u32 directly, the color will "bleed" into adjacent pixel
                *pixel_ptr.add(r_shift as usize / 8) = r;
                *pixel_ptr.add(g_shift as usize / 8) = g;
                *pixel_ptr.add(b_shift as usize / 8) = b;
            }
        }
    }

    /// VERY UNSAFE: the caller must ensure that the data is in the correct format (BGRA or RGBA
    /// depending on the framebuffer)
    pub const unsafe fn draw_wallpaper(&mut self, data: &[u32]) {
        // optimization: if wallpaper matches FB size exactly, use copy_nonoverlapping
        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr() as *const u8,
                self.buffer.ptr,
                (self.info.width * self.info.height * 4) as usize,
            );
        }
    }

    // pub fn write_byte(&mut self, byte: u8) {
    //     match byte {
    //         b'\n' => self.new_line(),
    //         byte => {
    //             if self.column_position >= BUFFER_WIDTH {
    //                 self.new_line();
    //             }
    //
    //             let row = BUFFER_HEIGHT - 1;
    //             let col = self.column_position;
    //             let color_code = self.color_code;
    //
    //             self.buffer.chars[row][col].write_volatile(ScreenChar {
    //                 ascii_character: byte,
    //                 color_code,
    //             });
    //
    //             self.column_position += 1;
    //         }
    //     }
    // }
}

pub struct FrameBufferInfo {
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u8,
    pub pitch: u32,
    pub r_shift: u8,
    pub g_shift: u8,
    pub b_shift: u8,
}

impl FrameBufferInfo {
    pub const fn compose_color(&self, r: u8, g: u8, b: u8) -> u32 {
        let &Self {
            r_shift: rs,
            g_shift: gs,
            b_shift: bs,
            ..
        } = self;

        ((r as u32) << rs) | ((g as u32) << gs) | ((b as u32) << bs)
    }
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub ptr: *mut u8,
}

/// SAFETY: We're using a mutex to ensure that only one thread can write to the framebuffer at a
/// time, so it's safe to send it across threads.
unsafe impl Send for FrameBuffer {}

impl FrameBuffer {}
