use crate::{fd::FileDescriptor, scheduler};

#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_close(fd: FileDescriptor) -> isize {
    log::debug!("Enter sys_close");

    scheduler::remove_io_interface(fd)
        .map_err(|e| -num::ToPrimitive::to_isize(&e).unwrap())
        .map(|_| 0)
        .unwrap()
}
