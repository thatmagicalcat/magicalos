use core::fmt::Debug;

mod error;

use alloc::{string::String, vec::Vec};
pub use error::Error;

use crate::fs::SeekFrom;

pub type Result<T> = core::result::Result<T, Error>;

pub trait IoInterface: Sync + Send + Debug {
    /// Attempts to read `len` bytes from the object pointed by the descriptor
    fn read(&self, _buf: &mut [u8]) -> Result<usize> {
        log::error!("No read implementation");
        Err(Error::NotImplemented)
    }

    /// Attempts to write `len` bytes to the object referenced by the descriptor
    fn write(&self, _buf: &[u8]) -> Result<usize> {
        log::error!("No write implementation");
        Err(Error::NotImplemented)
    }

    /// Attempts to change the current position of the file descriptor by `offset` bytes, in the
    /// direction specified by `seek_from`
    fn seek(&self, _offset: SeekFrom) -> Result<usize> {
        log::error!("No seek implementation");
        Err(Error::NotImplemented)
    }

    // TODO: file stat
}

/// This trait allows reading bytes from a source.
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    // Read all bytes until EOF
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let mut total_read = 0;

        loop {
            let mut probe = [0u8; 32];
            let bytes_read = self.read(&mut probe)?;

            if bytes_read == 0 {
                break; // EOF
            }

            buf.extend_from_slice(&probe[..bytes_read]);
            total_read += bytes_read;
        }

        Ok(total_read)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.read_to_end(unsafe { buf.as_mut_vec() })
    }
}

/// This trait allows writing bytes to a destination.
/// derived from Rust's std library
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Attempts to write the entire buffer into this writer
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            let bytes_written = self.write(buf)?;

            if bytes_written == 0 {
                return Err(Error::WriteAllEOF);
            }

            buf = &buf[bytes_written..];
        }

        Ok(())
    }

    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> Result<()> {
        struct Adapter<'a, T: ?Sized + 'a> {
            inner: &'a mut T,
            error: Result<()>,
        }

        impl<'a, T: 'a + ?Sized + Write> core::fmt::Write for Adapter<'a, T> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(core::fmt::Error)
                    }
                }
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(()),
        };

        match core::fmt::write(&mut output, args) {
            Ok(()) => Ok(()),
            Err(..) => {
                if output.error.is_err() {
                    output.error
                } else {
                    panic!("write_fmt failed without an underlying write error")
                }
            },
        }
    }
}
