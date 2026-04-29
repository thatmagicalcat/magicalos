use crate::{errno, fd, fs::SeekFrom};

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_seek(fd: fd::FileDescriptor, offset: isize, whence: usize) -> isize {
    log::trace!("Enter sys_seek, fd: {fd}");

    let seek_from = match whence {
        0 if offset >= 0 => SeekFrom::Start(offset as usize),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),

        _ => return -errno::EINVAL as _,
    };

    fd::seek(fd, seek_from)
        .map_or_else(|e| -num::ToPrimitive::to_isize(&e).unwrap(), |v| v as isize)
}
