use crate::{
    drivers,
    io::{self, IoInterface},
    memory,
};

#[derive(Debug)]
pub struct WebcamDevice;

impl IoInterface for WebcamDevice {
    fn mmap(&self, offset: usize) -> io::Result<memory::paging::PhysicalAddress> {
        let (phys_addr, size) = drivers::ivshmem::SHM_PHYS_ADDR.lock().ok_or_else(|| {
            log::error!("Webcam device is not initialized yet");
            io::Error::NoSuchDevice
        })?;

        if offset >= size {
            return Err(io::Error::InvalidValue);
        }

        Ok(memory::paging::PhysicalAddress(
            (phys_addr + offset as u64) as _,
        ))
    }
}
