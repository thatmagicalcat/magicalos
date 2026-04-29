use core::ffi::CStr;

use crate::{errno, fs};

// TODO: mode?
#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_mkdir(path: *const i8, _mode: u32) -> isize {
    // log::trace!("Enter sys_mkdir");

    let Ok(path_str) = (unsafe { CStr::from_ptr(path).to_str() }) else {
        return -errno::EINVAL as _;
    };

    log::trace!("Enter sys_mkdir: {path_str}");

    match fs::mkdir(path_str) {
        Ok(()) => 0,
        Err(e) => {
            log::warn!("sys_mkdir(): {e}");
            -num::ToPrimitive::to_isize(&e).unwrap_or(errno::EINVAL as _)
        }
    }
}
