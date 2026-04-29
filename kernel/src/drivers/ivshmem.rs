use crate::{
    bus::pci::{DriverError, PciBar, PciDevice, PciDriver, PciMatcher},
    synch::Spinlock,
};

pub static SHM_PHYS_ADDR: Spinlock<Option<(u64, usize)>> = Spinlock::new(None);

pub struct IvShMemDriver;
impl PciDriver for IvShMemDriver {
    fn name(&self) -> &'static str {
        "ivshmem-plain"
    }

    fn ids(&self) -> &[crate::bus::pci::PciMatcher] {
        &[PciMatcher::Id {
            vendor: 0x1AF4,
            device: 0x1110,
        }]
    }

    fn probe(&self, device: &PciDevice) -> Result<(), DriverError> {
        log::info!("Loading {} PCI Driver", self.name());

        // BAR0 holds device registers (256 Byte MMIO)
        // BAR1 holds MSI-X table and PBA (only ivshmem-doorbell)
        // BAR2 maps the shared memory object
        let shm_bar = device.get_bar(2).ok_or(DriverError::BarNotFound)?;

        let (p_addr, size) = match shm_bar {
            PciBar::Mmio32 { address, size, .. } => (address as u64, size),
            PciBar::Mmio64 { address, size, .. } => (address, size),
            _ => return Err(DriverError::InvalidConfiguration),
        };

        log::debug!("SHM p_addr: {p_addr:#X}, size: {} KiB", size / 1024);
        device.enable_capabilities(true, false, false);
        *SHM_PHYS_ADDR.lock() = Some((p_addr, size));

        Ok(())
    }
}
