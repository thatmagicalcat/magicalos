//! source: https://wiki.osdev.org/PCI

use crate::bus::port::Port;

use alloc::{boxed::Box, vec::Vec};

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

pub struct PciDriverManager {
    // We keep the drivers alive here so they can
    // handle interrupts or state later.
    drivers: Vec<Box<dyn PciDriver + Send>>,
}

impl PciDriverManager {
    pub const fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    /// Add a driver to the registry and immediately probe existing devices.
    pub fn add_and_probe(&mut self, devices: &[PciDevice], driver: Box<dyn PciDriver + Send>) {
        let matchers = driver.ids();

        for dev in devices {
            for matcher in matchers {
                if matcher.match_pci(dev)
                    && let Err(e) = driver.probe(dev)
                {
                    log::error!(
                        "Driver {} failed to probe device {dev}: {e}",
                        driver.name(),
                    );

                    continue;
                }
            }
        }

        self.drivers.push(driver);
    }
}

impl Default for PciDriverManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DriverError {
    NoMemory,
    BarMappingFailed,
    BarNotFound,
    DeviceNotSupported,
    InvalidConfiguration,
    HardwareFault,
}

impl core::fmt::Display for DriverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoMemory => write!(f, "Out of memory during driver init"),
            Self::BarMappingFailed => write!(f, "Failed to map PCI BARs"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl core::error::Error for DriverError {}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PciMatcher {
    /// Match a specific hardware (e.g., specific Realtek card)
    Id { vendor: u16, device: u16 },
    /// Match a category (e.g., any AHCI controller)
    Class {
        class: u8,
        subclass: u8,
        prog_if: u8,
    },
}

impl PciMatcher {
    const fn match_pci(&self, dev: &PciDevice) -> bool {
        match self {
            PciMatcher::Id { vendor, device } => {
                dev.vendor_id == *vendor && dev.device_id == *device
            }
            PciMatcher::Class {
                class,
                subclass,
                prog_if,
            } => dev.class == *class && dev.subclass == *subclass && dev.prog_if == *prog_if,
        }
    }
}

pub trait PciDriver {
    /// A human-readable name for the driver
    fn name(&self) -> &'static str;

    /// Return the list of devices this driver supports
    fn ids(&self) -> &[PciMatcher];

    /// Called by the kernel when a matching device is found.
    /// The driver should map BARs and initialize the hardware here.
    fn probe(&self, device: &PciDevice) -> Result<(), DriverError>;
}

#[derive(Debug, Clone, Copy)]
pub enum PciBar {
    None,
    Mmio32 {
        address: u32,
        size: usize,
        prefetchable: bool,
    },
    Mmio64 {
        address: u64,
        size: usize,
        prefetchable: bool,
    },
    Io {
        port: u16,
        size: usize,
    },
}

#[derive(Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub is_multi_function: bool,
    pub header_type: u8,
}

impl PciDevice {
    pub fn enable_capabilities(&self, mmio: bool, dma: bool, io: bool) {
        let mut command = self.read_u32(0x4);

        if mmio {
            command |= 1 << 1;
        }

        if dma {
            command |= 1 << 2;
        }

        if io {
            command |= 1;
        }

        self.write_u32(0x4, command);
    }

    pub fn read_u32(&self, offset: u8) -> u32 {
        read_u32(self.bus, self.slot, self.func, offset)
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        write_u32(self.bus, self.slot, self.func, offset, value);
    }

    pub fn get_bar(&self, bar_index: u8) -> Option<PciBar> {

        match self.header_type {
            0 => {
                if bar_index >= 6 {
                    return None;
                }
            }
            1 => {
                if bar_index >= 2 {
                    return None;
                }
            }

            _ => return None,
        }

        let offset = 0x10 + bar_index * 4;

        let old_command = read_u32(self.bus, self.slot, self.func, 0x04);
        // clear bit 0 (I/O), bit 1 (Memory), and bit 2 (Bus Master)
        self.write_u32(0x4, old_command & !0b111);

        let bar_val = self.read_u32(offset);

        if bar_val == 0 {
            return None;
        }

        let is_io = (bar_val & 0x1) != 0;

        self.write_u32(offset, 0xFFFF_FFFF);
        let size_mask = self.read_u32(offset);
        self.write_u32(offset, bar_val); // restore original

        let pci_bar = if is_io {
            PciBar::Io {
                port: (bar_val & 0xFFFF_FFFC) as _,
                size: !(size_mask as usize & 0xFFFF_FFFC) + 1,
            }
        } else {
            let is_64bit = (bar_val & 0x6) == 0x4;
            let prefetchable = (bar_val & 0x8) != 0;

            if is_64bit && bar_index < 5 {
                let next_offset = offset + 4;
                let bar_val_hi = self.read_u32(next_offset);

                self.write_u32(next_offset, 0xFFFF_FFFF);
                let size_mask_hi = self.read_u32(next_offset);
                self.write_u32(next_offset, bar_val_hi);

                let address = ((bar_val_hi as u64) << 32) | (bar_val & 0xFFFF_FFF0) as u64;
                let full_mask = ((size_mask_hi as u64) << 32) | (size_mask & 0xFFFF_FFF0) as u64;
                let size = (!full_mask + 1) as usize;

                PciBar::Mmio64 {
                    address,
                    size,
                    prefetchable,
                }
            } else {
                let address = bar_val & 0xFFFF_FFF0;
                let size = (!(size_mask & 0xFFFF_FFF0) + 1) as usize;

                PciBar::Mmio32 {
                    address,
                    size,
                    prefetchable,
                }
            }
        };

        self.write_u32(0x4, old_command);

        Some(pci_bar)
    }
}

impl core::fmt::Display for PciDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let PciDevice {
            bus,
            slot,
            func,
            vendor_id,
            device_id,
            class,
            subclass,
            prog_if,
            ..
        } = self;

        let class_enum = PciClass::from_u8(*class);
        let subclass_name = class_enum
            .map(|i| i.get_subclass_name(*subclass))
            .unwrap_or("Unknown");

        write!(
            f,
            "{bus:02x}:{slot:02x}.{func} | {subclass_name} ({class_enum:?}, {prog_if}) | Vendor: ({vendor_id:#06x}) | Device: {device_id:#06x}",
        )
    }
}

pub fn read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | (offset as u32 & 0xFC);

    unsafe {
        u32::write_to_port(CONFIG_ADDRESS, address);
        u32::read_from_port(CONFIG_DATA)
    }
}

pub fn write_u32(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    let address = (1 << 31)
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | (offset as u32 & 0xFC);

    unsafe {
        u32::write_to_port(CONFIG_ADDRESS, address);
        u32::write_to_port(CONFIG_DATA, value);
    }
}

pub fn enumerate() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..=255 {
        for slot in 0..=31 {
            if let Some(dev0) = check_function(bus, slot, 0) {
                let is_multi = dev0.is_multi_function;
                devices.push(dev0);

                // check functions 1-7
                if is_multi {
                    for func in 1..=7 {
                        if let Some(dev) = check_function(bus, slot, func) {
                            devices.push(dev);
                        }
                    }
                }
            }
        }
    }
    devices
}

fn check_function(bus: u8, slot: u8, func: u8) -> Option<PciDevice> {
    let reg0 = read_u32(bus, slot, func, 0);
    let vendor_id = (reg0 & 0xFFFF) as u16;

    // 0xFFFF means no device is present
    if vendor_id == 0xFFFF {
        return None;
    }

    let reg8 = read_u32(bus, slot, func, 0x08);
    let reg_c = read_u32(bus, slot, func, 0x0C);

    // Store the raw header_type but provide a way to mask it
    let raw_header_type = ((reg_c >> 16) & 0xFF) as u8;

    Some(PciDevice {
        bus,
        slot,
        func,
        header_type: raw_header_type & 0x7F, // Mask out the multi-function bit
        is_multi_function: (raw_header_type & 0x80) != 0, // Store this separately if needed
        vendor_id,
        device_id: (reg0 >> 16) as u16,
        class: (reg8 >> 24) as u8,
        subclass: (reg8 >> 16) as u8,
        prog_if: (reg8 >> 8) as u8,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PciClass {
    Unclassified = 0x00,
    MassStorageController = 0x01,
    NetworkController = 0x02,
    DisplayController = 0x03,
    MultimediaController = 0x04,
    MemoryController = 0x05,
    Bridge = 0x06,
    SimpleCommunicationController = 0x07,
    BaseSystemPeripheral = 0x08,
    InputDeviceController = 0x09,
    DockingStation = 0x0A,
    Processor = 0x0B,
    SerialBusController = 0x0C,
    WirelessController = 0x0D,
    IntelligentController = 0x0E,
    SatelliteCommunicationController = 0x0F,
    EncryptionController = 0x10,
    SignalProcessingController = 0x11,
    ProcessingAccelerator = 0x12,
    NonEssentialInstrumentation = 0x13,
    Coprocessor = 0x40,
    UnassignedClass = 0xFF,
}

impl PciClass {
    pub const fn from_u8(value: u8) -> Option<Self> {
        use PciClass::*;

        Some(match value {
            0x00 => Unclassified,
            0x01 => MassStorageController,
            0x02 => NetworkController,
            0x03 => DisplayController,
            0x04 => MultimediaController,
            0x05 => MemoryController,
            0x06 => Bridge,
            0x07 => SimpleCommunicationController,
            0x08 => BaseSystemPeripheral,
            0x09 => InputDeviceController,
            0x0A => DockingStation,
            0x0B => Processor,
            0x0C => SerialBusController,
            0x0D => WirelessController,
            0x0E => IntelligentController,
            0x0F => SatelliteCommunicationController,
            0x10 => EncryptionController,
            0x11 => SignalProcessingController,
            0x12 => ProcessingAccelerator,
            0x13 => NonEssentialInstrumentation,
            0x40 => Coprocessor,
            0xFF => UnassignedClass,

            _ => return None,
        })
    }

    pub const fn get_subclass_name(&self, subclass: u8) -> &'static str {
        use PciClass::*;

        match self {
            Unclassified => match subclass {
                0x00 => "Non-VGA-Compatible Device",
                0x01 => "VGA-Compatible Device",

                _ => "Unknown Subclass",
            },

            MassStorageController => match subclass {
                0x00 => "SCSI Controller",
                0x01 => "IDE Controller",
                0x02 => "Floppy Disk Controller",
                0x03 => "IPI Bus Controller",
                0x04 => "RAID Controller",
                0x05 => "ATA Controller",
                0x06 => "Serial ATA Controller",
                0x07 => "Serial Attached SCSI Controller",
                0x08 => "Non-Volatile Memory Controller",
                0x80 => "Other Mass Storage Controller",

                _ => "Unknown Subclass",
            },

            NetworkController => match subclass {
                0x00 => "Ethernet Controller",
                0x01 => "Token Ring Controller",
                0x02 => "FDDI Controller",
                0x03 => "ATM Controller",
                0x04 => "ISDN Controller",
                0x05 => "WorldFip Controller",
                0x06 => "PICMG 2.14 Multi Computing",
                0x07 => "Infiniband Controller",
                0x80 => "Other Network Controller",

                _ => "Unknown Subclass",
            },

            DisplayController => match subclass {
                0x00 => "VGA-Compatible Controller",
                0x01 => "XGA Controller",
                0x02 => "3D Controller",
                0x80 => "Other Display Controller",

                _ => "Unknown Subclass",
            },

            MultimediaController => match subclass {
                0x00 => "Multimedia Video Controller",
                0x01 => "Multimedia Audio Controller",
                0x02 => "Computer Telephony Device",
                0x03 => "Audio Device",
                0x80 => "Other Multimedia Controller",

                _ => "Unknown Subclass",
            },

            MemoryController => match subclass {
                0x00 => "RAM Controller",
                0x01 => "Flash Controller",
                0x80 => "Other Memory Controller",

                _ => "Unknown Subclass",
            },

            Bridge => match subclass {
                0x00 => "Host Bridge",
                0x01 => "ISA Bridge",
                0x02 => "EISA Bridge",
                0x03 => "MCA Bridge",
                0x04 => "PCI-to-PCI Bridge",
                0x05 => "PCMCIA Bridge",
                0x06 => "NuBus Bridge",
                0x07 => "CardBus Bridge",
                0x08 => "RACEway Bridge",
                0x09 => "Semi-transparent PCI-to-PCI Bridge",
                0x0A => "InfiniBand-to-PCI Host Bridge",
                0x80 => "Other Bridge Device",

                _ => "Unknown Subclass",
            },

            SimpleCommunicationController => match subclass {
                0x00 => "Serial Controller",
                0x01 => "Parallel Controller",
                0x02 => "Multiport Serial Controller",
                0x03 => "Modem",
                0x04 => "IEEE 488.1/2 (GPIB) Controller",
                0x05 => "Smart Card Controller",
                0x80 => "Other Simple Communication Controller",

                _ => "Unknown Subclass",
            },

            BaseSystemPeripheral => match subclass {
                0x00 => "PIC",
                0x01 => "DMA Controller",
                0x02 => "Timer",
                0x03 => "RTC Controller",
                0x04 => "PCI Hot-Plug Controller",
                0x05 => "SD Host Controller",
                0x06 => "IOMMU",
                0x80 => "Other Base System Peripheral",

                _ => "Unknown Subclass",
            },

            InputDeviceController => match subclass {
                0x00 => "Keyboard Controller",
                0x01 => "Digitizer Pen",
                0x02 => "Mouse Controller",
                0x03 => "Scanner Controller",
                0x04 => "Gameport Controller",
                0x80 => "Other Input Device Controller",

                _ => "Unknown Subclass",
            },

            DockingStation => match subclass {
                0x00 => "Generic Docking Station",
                0x80 => "Other Docking Station",

                _ => "Unknown Subclass",
            },

            Processor => match subclass {
                0x00 => "386",
                0x01 => "486",
                0x02 => "Pentium",
                0x03 => "Alpha",
                0x04 => "PowerPC",
                0x05 => "MIPS",
                0x06 => "Co-Processor",
                0x80 => "Other Processor",

                _ => "Unknown Subclass",
            },

            SerialBusController => match subclass {
                0x00 => "FireWire (IEEE 1394) Controller",
                0x01 => "ACCESS Bus",
                0x02 => "SSA",
                0x03 => "USB Controller",
                0x04 => "Fibre Channel",
                0x05 => "SMBus",
                0x06 => "InfiniBand",
                0x07 => "IPMI Interface",
                0x08 => "SERCOS Interface (IEC 61491)",
                0x09 => "CANbus",
                0x80 => "Other Serial Bus Controller",

                _ => "Unknown Subclass",
            },

            WirelessController => match subclass {
                0x00 => "iRDA Compatible Controller",
                0x01 => "Consumer IR Controller",
                0x10 => "RF Controller",
                0x11 => "Bluetooth Controller",
                0x12 => "Broadband Controller",
                0x20 => "Ethernet Controller (802.11a/b/g)",
                0x21 => "Ethernet Controller (802.16)",
                0x80 => "Other Wireless Controller",

                _ => "Unknown Subclass",
            },

            IntelligentController => match subclass {
                0x00 => "I20 Architecture",

                _ => "Unknown Subclass",
            },

            SatelliteCommunicationController => match subclass {
                0x01 => "Satellite TV Controller",
                0x02 => "Satellite Audio Controller",
                0x03 => "Satellite Voice Controller",
                0x04 => "Satellite Data Controller",

                _ => "Unknown Subclass",
            },

            EncryptionController => match subclass {
                0x00 => "Network and Computing Encrytion/Decryption",
                0x10 => "Entertainment Encryption/Decryption",
                0x80 => "Other Encryption Controller",

                _ => "Unknown Subclass",
            },

            SignalProcessingController => match subclass {
                0x00 => "DPIO Modules",
                0x01 => "Performance Counters",
                0x10 => "Communications Synchronizer",
                0x20 => "Signal Processing Management",
                0x80 => "Other Signal Processing Controller",

                _ => "Unknown Subclass",
            },

            _ => "Unknown Subclass",
        }
    }
}
