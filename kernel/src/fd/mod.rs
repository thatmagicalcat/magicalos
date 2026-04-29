use crate::{io, memory::{self, paging::PhysicalAddress}, scheduler};

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

pub(crate) fn read(fd: i32, buf: &mut [u8]) -> io::Result<usize> {
    let obj = scheduler::get_io_interface(fd)?;

    if buf.is_empty() {
        return Ok(0);
    }

    obj.read(buf)
}

pub(crate) fn mmap(fd: i32, offset: usize) -> io::Result<PhysicalAddress> {
    let obj = scheduler::get_io_interface(fd)?;
    let phys_addr = obj.mmap(offset)?;

    assert!(phys_addr.0.is_multiple_of(memory::PAGE_SIZE as _), "Unaligned physical address");

    Ok(phys_addr)
}

pub(crate) fn seek(fd: FileDescriptor, seek: crate::fs::SeekFrom) -> io::Result<usize> {
    let obj = scheduler::get_io_interface(fd)?;
    obj.seek(seek)
}
