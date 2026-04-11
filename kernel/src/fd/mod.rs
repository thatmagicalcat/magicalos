use crate::io;

pub type FileDescriptor = i32;

pub const STDIN_FILENO: FileDescriptor = 0;
pub const STDOUT_FILENO: FileDescriptor = 1;
pub const STDERR_FILENO: FileDescriptor = 2;

pub(crate) fn write(fd: FileDescriptor, buf: &[u8]) -> io::Result<usize> {
    todo!()
}
