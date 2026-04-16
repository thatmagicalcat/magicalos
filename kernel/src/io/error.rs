use num_derive::{FromPrimitive, ToPrimitive};

use crate::errno;

#[derive(Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(isize)]
pub(crate) enum Error {
    NotImplemented = errno::ENOIMPL as _,
    NoSuchFileOrDirectory = errno::ENOENT as _,
    InvalidValue = errno::EINVAL as _,
    BadFileDescriptor = errno::EBADF as _,
    InvalidArgument = errno::ENOINVARG as _,
    NotADirectory = errno::ENOTDIR as _,
    InvalidFsPath = errno::ENOINVPATH as _,
    TooManyOpenFiles = errno::EMFILE as _,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for Error {}
