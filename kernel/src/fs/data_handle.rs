use spin::RwLock;
use alloc::{sync::Arc, vec::Vec};

use super::SeekFrom;
use crate::{fs::OpenOptions, io::{self, IoInterface}, synch::Spinlock};

#[derive(Debug)]
pub(crate) struct StaticData {
    pos: Spinlock<usize>,
    data: Arc<RwLock<&'static [u8]>>,
}

impl StaticData {
    pub fn new(data: &'static [u8]) -> Self {
        Self {
            pos: Spinlock::new(0),
            data: Arc::new(RwLock::new(data)),
        }
    }

    pub fn get_handle(&self, _flags: OpenOptions) -> StaticData {
        StaticData {
            pos: Spinlock::new(0),
            data: Arc::clone(&self.data),
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut pos = self.pos.lock();
        let data: spin::RwLockReadGuard<&'static [u8]> = self.data.read();

        if *pos >= data.len() {
            return Ok(0);
        }

        let len;
        if data.len() - *pos >= buf.len() {
            len = buf.len();
        } else {
            len = data.len() - *pos;
        }

        buf[0..len].copy_from_slice(&data[*pos..*pos + len]);
        *pos += len;

        Ok(len)
    }

    pub fn seek(&self, seek: SeekFrom) -> io::Result<usize> {
        let mut pos = self.pos.lock();

        match seek {
            SeekFrom::Start(n) => *pos = n,
            SeekFrom::Current(n) => {
                let len = self.data.read().len() as isize;
                let new_pos = len + n;

                if new_pos < 0 {
                    return Err(io::Error::InvalidValue);
                }
            }

            SeekFrom::End(n) => {
                let len = self.data.read().len() as isize;
                let new_pos = len + n;

                if new_pos < 0 {
                    return Err(io::Error::InvalidValue);
                }
            }
        }

        Ok(*pos)
    }

    pub fn len(&self) -> usize {
        self.data.read().len()
    }
}

impl Clone for StaticData {
    fn clone(&self) -> Self {
        Self {
            pos: Spinlock::new(*self.pos.lock()),
            data: Arc::clone(&self.data),
        }
    }
}

#[derive(Debug)]
pub(crate) struct DynamicData {
    writable: bool,
    pos: Spinlock<usize>,
    data: Arc<RwLock<Vec<u8>>>,
}

impl DynamicData {
    pub fn new(writable: bool) -> Self {
        Self {
            writable,
            pos: Spinlock::new(0),
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_handle(&self, flags: OpenOptions) -> DynamicData {
        let pos = if flags.contains(OpenOptions::APPEND) {
            self.data.read().len()
        } else {
            0
        };

        DynamicData {
            writable: flags.contains(OpenOptions::WRONLY | OpenOptions::APPEND),
            pos: Spinlock::new(pos),
            data: Arc::clone(&self.data),
        }
    }

    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut pos = self.pos.lock();
        let data: spin::RwLockReadGuard<Vec<u8>> = self.data.read();

        if *pos >= data.len() {
            return Ok(0);
        }

        let len;
        if data.len() - *pos >= buf.len() {
            len = buf.len();
        } else {
            len = data.len() - *pos;
        }

        buf[0..len].copy_from_slice(&data[*pos..*pos + len]);
        *pos += len;

        Ok(len)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        if !self.writable {
            return Err(io::Error::BadFileDescriptor);
        }

        let mut data: spin::RwLockWriteGuard<Vec<u8>> = self.data.write();
        let mut pos = self.pos.lock();

        // reserve space for write
        if *pos + buf.len() > data.len() {
            data.resize(*pos + buf.len(), 0);
        }

        data[*pos..*pos + buf.len()].copy_from_slice(buf);
        *pos += buf.len();

        Ok(buf.len())
    }

    pub fn seek(&self, seek: SeekFrom) -> io::Result<usize> {
        let mut pos = self.pos.lock();

        match seek {
            SeekFrom::Start(n) => *pos = n,
            SeekFrom::Current(n) => {
                let len = self.data.read().len() as isize;
                let new_pos = len + n;

                if new_pos < 0 {
                    return Err(io::Error::InvalidValue);
                }
            }

            SeekFrom::End(n) => {
                let len = self.data.read().len() as isize;
                let new_pos = len + n;

                if new_pos < 0 {
                    return Err(io::Error::InvalidValue);
                }
            }
        }

        Ok(*pos)
    }
}

impl Clone for DynamicData {
    fn clone(&self) -> Self {
        Self {
            writable: self.writable,
            pos: Spinlock::new(*self.pos.lock()),
            data: Arc::clone(&self.data),
        }
    }
}

#[derive(Debug)]
pub(crate) enum DataHandle {
    Dynamic(DynamicData),
    Static(StaticData),
}

impl DataHandle {
    pub fn get_handle(&self, flags: OpenOptions) -> DataHandle {
        match self {
            DataHandle::Dynamic(data) => DataHandle::Dynamic(data.get_handle(flags)),
            DataHandle::Static(data) => DataHandle::Static(data.get_handle(flags)),
        }
    }
}
