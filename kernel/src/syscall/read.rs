use crate::fd::FileDescriptor;

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_read(fd: FileDescriptor, buf: *mut u8, len: usize) -> isize {
    log::trace!("Enter sys_read");

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    crate::fd::read(fd, slice)
        .map_or_else(|e| -num::ToPrimitive::to_isize(&e).unwrap(), |v| v as _)
}
