use core::mem::ManuallyDrop;

use alloc::string::String;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_write(s: *const u8, len: usize) -> isize {
    log::debug!("Enter syswrite");

    // SAFETY: casting *const u8 -> *mut u8 is actually safe because we are not making any changes
    let str = ManuallyDrop::new(unsafe { String::from_raw_parts(s as *mut u8, len, len) });
    crate::print!("{}", *str);
    len as _
}
