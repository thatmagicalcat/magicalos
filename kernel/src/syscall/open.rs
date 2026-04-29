use core::ffi::CStr;
use crate::{errno, fs};

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_open(path: *const i8, flags: u32, _mode: u32) -> isize {
    log::trace!("Enter sys_open");

    let Ok(path_str) = (unsafe { CStr::from_ptr(path).to_str() }) else {
        return -errno::EINVAL as _;
    };

    match fs::open(path_str, fs::OpenOptions::from_bits_truncate(flags)) {
        Ok(fd) => fd as isize,
        Err(e) => {
            log::error!("sys_open(): {e}");
            -num::ToPrimitive::to_isize(&e).unwrap_or(errno::EINVAL as _)
        }
    }
}
