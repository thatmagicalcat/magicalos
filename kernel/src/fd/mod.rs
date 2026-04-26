use alloc::sync::Arc;

use crate::{io, scheduler};

pub mod generic;

pub type FileDescriptor = i32;

pub const STDIN_FILENO: FileDescriptor = 0;
pub const STDOUT_FILENO: FileDescriptor = 1;
pub const STDERR_FILENO: FileDescriptor = 2;

pub(crate) fn write(fd: FileDescriptor, buf: &[u8]) -> io::Result<usize> {
    let obj = scheduler::get_io_interface(fd)?;

    if buf.is_empty() {
        return Ok(0);
    }

    obj.write(buf)
}

pub(crate) fn read(fd: i32, buf: &mut [u8]) -> Result<usize, io::Error> {
    let obj = scheduler::get_io_interface(fd)?;

    if buf.is_empty() {
        return Ok(0);
    }

    obj.read(buf)
}
