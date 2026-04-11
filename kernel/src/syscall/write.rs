use crate::fd::FileDescriptor;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_write(fd: FileDescriptor, buf: *mut u8, len: usize) -> isize {
    log::debug!("Enter syswrite");

    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
    crate::fd::write(fd, slice)
        .map_or_else(|e| -num::ToPrimitive::to_isize(&e).unwrap(), |v| v as _)
}
