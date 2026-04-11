use core::fmt::Debug;

mod error;

pub(crate) use error::Error;

pub type Result<T> = core::result::Result<T, Error>;

pub(crate) trait IoInterface: Sync + Send + Debug {
    /// Attempts to read `len` bytes from the object pointed by the descriptor
    fn read(&self, _buf: &mut [u8]) -> Result<usize> {
        Err(Error::NotImplemented)
    }

    /// Attempts to write `len` bytes to the object referenced by the descriptor
    fn write(&self, _buf: &[u8]) -> Result<usize> {
        Err(Error::NotImplemented)
    }
}
