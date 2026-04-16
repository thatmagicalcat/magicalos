use alloc::string::String;

use crate::fd::FileDescriptor;

#[derive(Debug)]
pub struct File {
    fd: FileDescriptor,
    path: String,
}

