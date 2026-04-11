//! source: https://wiki.osdev.org/PCI

use crate::bus::port::Port;
use alloc::vec::Vec;
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

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
        } = self;

        let class_enum = PciClass::from_u8(*class).expect("Unknown PCI class");
        let subclass_name = class_enum.get_subclass_name(*subclass);

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

pub fn enumerate() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..=255 {
        for slot in 0..=31 {
            // Read Vendor ID from function 0
            let reg0 = read_u32(bus, slot, 0, 0);
            let vendor_id = (reg0 & 0xFFFF) as u16;
            if vendor_id != 0xFFFF {
                // Device exists! Check if multi-function
                check_function(&mut devices, bus, slot, 0);

                let header_type = (read_u32(bus, slot, 0, 0x0C) >> 16) as u8;
                if (header_type & 0x80) != 0 {
                    // Multi-function, check functions 1-7
                    for func in 1..=7 {
                        let reg0 = read_u32(bus, slot, func, 0);
                        if (reg0 & 0xFFFF) as u16 != 0xFFFF {
                            check_function(&mut devices, bus, slot, func);
                        }
                    }
                }
            }
        }
    }
    devices
}

fn check_function(devices: &mut Vec<PciDevice>, bus: u8, slot: u8, func: u8) {
    let reg0 = read_u32(bus, slot, func, 0);
    let reg8 = read_u32(bus, slot, func, 0x08);

    devices.push(PciDevice {
        bus,
        slot,
        func,
        vendor_id: (reg0 & 0xFFFF) as u16,
        device_id: (reg0 >> 16) as u16,
        class: (reg8 >> 24) as u8,
        subclass: (reg8 >> 16) as u8,
        prog_if: (reg8 >> 8) as u8,
    });
}
