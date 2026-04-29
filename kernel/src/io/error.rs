use num_derive::{FromPrimitive, ToPrimitive};

use crate::errno;

#[derive(Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(isize)]
pub enum Error {
    NotImplemented = errno::ENOSYS as _,
    NoSuchFileOrDirectory = errno::ENOENT as _,
    InvalidValue = errno::EINVAL as _,
    BadFileDescriptor = errno::EBADF as _,
    NotADirectory = errno::ENOTDIR as _,
    NotAFile = errno::EISDIR as _,
    NoSuchDevice = errno::ENODEV as _,
    TooManyOpenFiles = errno::EMFILE as _,
    WriteAllEOF = errno::EPIPE as _,
    AlreadyExists = errno::EEXIST as _,
    StaleId = errno::ESTALE as _,
    DirectoryNotEmpty = errno::ENOTEMPTY as _,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
