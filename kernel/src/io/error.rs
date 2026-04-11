use num_derive::{FromPrimitive, ToPrimitive};

use crate::errno;

#[derive(Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(isize)]
pub(crate) enum Error {
    NotImplemented = errno::ENOIMPL as _,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NotImplemented => write!(f, "Not implemented")
        }
    }
}

impl core::error::Error for Error {}
