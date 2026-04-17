use alloc::string::String;

use crate::{
    fd::{self, FileDescriptor},
    fs::{self, OpenOptions},
    io, scheduler,
};

#[derive(Debug)]
pub struct File {
    fd: FileDescriptor,
    path: String,
}

impl File {
    pub fn create(path: &str) -> io::Result<Self> {
        let fd = fs::open(path, OpenOptions::CREATE)?;

        Ok(Self {
            fd,
            path: String::from(path),
        })
    }

    /// open file in read-write mode
    pub fn open(path: &str) -> io::Result<Self> {
        let fd = fs::open(path, OpenOptions::RW)?;

        Ok(Self {
            fd,
            path: String::from(path),
        })
    }
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        fd::write(self.fd, buf)
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        fd::read(self.fd, buf)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        _ = scheduler::remove_io_interface(self.fd);
    }
}
