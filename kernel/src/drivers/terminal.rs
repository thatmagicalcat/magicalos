use core::{alloc::Layout, ffi, ptr};

use alloc::alloc::{alloc, dealloc};

use crate::limine_requests::*;
use crate::synch::Spinlock;

pub static TERMINAL: Spinlock<Option<Terminal>> = Spinlock::new(None);

pub struct Terminal {
    ctx: *mut flanterm::flanterm_context,
}

/// SAFETY: trust me bro
unsafe impl Send for Terminal {}

pub fn init() {
    let mut terminal_lock = TERMINAL.lock();
    *terminal_lock = Some(Terminal::new());
}

impl Terminal {
    pub fn new() -> Self {
        let ctx = flanterm_console_init();

        if ctx.is_null() {
            log::error!("Failed to initialize terminal context");
        } else {
            log::info!("Flanterm Terminal context initialized");
        }

        Self { ctx }
    }

    pub const fn inner(&mut self) -> *mut flanterm::flanterm_context {
        self.ctx
    }

    pub fn write_bytes(&mut self, buf: &[u8]) {
        unsafe { flanterm::flanterm_write(self.ctx, buf.as_ptr() as _, buf.len()) };
    }
}

impl core::fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe { flanterm::flanterm_write(self.ctx, s.as_ptr() as _, s.len() as _) };
        Ok(())
    }
}

fn flanterm_console_init() -> *mut flanterm::flanterm_context {
    let fb = unsafe {
        assert!(!FRAMEBUFFER_REQUEST.response.is_null());

        let response = &*FRAMEBUFFER_REQUEST.response;

        assert!(response.framebuffer_count > 0, "No framebuffer found");
        assert!(
            !response.framebuffers.is_null(),
            "Framebuffers array pointer is null"
        );

        let first_fb = *response.framebuffers;
        assert!(!first_fb.is_null(), "First framebuffer pointer is null");

        &*first_fb
    };

    // let mut params = None::<limine::limine_flanterm_fb_init_params>;
    let mut params = unsafe {
        let resp = FLANTERM_FB_INIT_PARAMS_REQUEST.response;
        if resp.is_null() {
            log::error!("Flanterm init parameters are not provided by the bootloader");
            None
        } else if (*resp).entry_count == 0 || (*resp).entries.is_null() {
            log::error!("Flanterm init parameters entry list is empty or null");
            None
        } else {
            Some(**(*resp).entries)
        }
    };

    let (
        canvas,
        ansi_colours,
        ansi_bright_colours,
        default_bg,
        default_fg,
        default_bg_bright,
        default_fg_bright,
        font,
        font_width,
        font_height,
        font_spacing,
        font_scale_x,
        font_scale_y,
        margin,
        rotation,
    ) = if let Some(ref mut p) = params {
        (
            p.canvas,
            p.ansi_colours.as_mut_ptr(),
            p.ansi_bright_colours.as_mut_ptr(),
            &raw mut p.default_bg,
            &raw mut p.default_fg,
            &raw mut p.default_bg_bright,
            &raw mut p.default_fg_bright,
            p.font,
            p.font_width as _,
            p.font_height as _,
            p.font_spacing as _,
            p.font_scale_x as _,
            p.font_scale_y as _,
            p.margin as _,
            p.rotation as _,
        )
    } else {
        (
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
            1,
            0,
            0,
            0,
            0,
        )
    };

    unsafe {
        extern "C" fn kmalloc(size: usize) -> *mut ffi::c_void {
            unsafe { alloc(Layout::from_size_align_unchecked(size, 1)) as _ }
        }

        extern "C" fn kfree(ptr: *mut ffi::c_void, size: usize) {
            unsafe { dealloc(ptr as _, Layout::from_size_align_unchecked(size, 1)) };
        }

        log::info!(
            "Flanterm framebuffer: {}x{} @ {:#010p}",
            fb.width,
            fb.height,
            fb.address
        );

        flanterm::flanterm_fb_init(
            Some(kmalloc),
            Some(kfree),
            fb.address as _,
            fb.width as _,
            fb.height as _,
            fb.pitch as _,
            fb.red_mask_size as _,
            fb.red_mask_shift,
            fb.green_mask_size,
            fb.green_mask_shift,
            fb.blue_mask_size,
            fb.blue_mask_shift,
            canvas,
            ansi_colours,
            ansi_bright_colours,
            default_bg,
            default_fg,
            default_bg_bright,
            default_fg_bright,
            font,
            font_width,
            font_height,
            font_spacing,
            font_scale_x,
            font_scale_y,
            margin,
            rotation,
        )
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}

pub struct Reset;

impl core::fmt::Display for Reset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "\x1b[0m")
    }
}

pub struct Foreground(Color);
pub struct Background(Color);

impl Color {
    pub const fn fg(self) -> Foreground {
        Foreground(self)
    }

    pub const fn bg(self) -> Background {
        Background(self)
    }
}

impl core::fmt::Display for Foreground {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = match self.0 {
            Color::Black => 30,
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,
            Color::Default => 39,
        };

        write!(f, "\x1b[{}m", code)
    }
}

impl core::fmt::Display for Background {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = match self.0 {
            Color::Black => 40,
            Color::Red => 41,
            Color::Green => 42,
            Color::Yellow => 43,
            Color::Blue => 44,
            Color::Magenta => 45,
            Color::Cyan => 46,
            Color::White => 47,
            Color::Default => 49,
        };

        write!(f, "\x1b[{}m", code)
    }
}
