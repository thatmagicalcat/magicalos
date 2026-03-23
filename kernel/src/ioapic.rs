//! source: https://wiki.osdev.org/IOAPIC

use alloc::vec::Vec;
use core::ptr;

use bitflags::bitflags;
use spin::Mutex;

use crate::{
    memory::{
        self, Frame, FrameAllocator,
        paging::{EntryFlags, Mapper, VirtualAddress},
    },
    utils,
};

const IOREGSEL: usize = 0x00;
const IOWIN: usize = 0x10;

const IOAPICID_INDEX: u8 = 0x00;
/// This register (index 1) contains the I/O APIC Version in bits 0 - 7,
/// and the Max Redirection Entry which is "how many IRQs can this I/O
/// APIC handle - 1". It is encoded in bits 16 - 23.
const IOAPICVER_INDEX: u8 = 0x01;
const IOAPICARB_INDEX: u8 = 0x02;

const fn ioapic_redirection_table_offset(n: usize) -> usize {
    0x10 + 2 * n
}

pub static IOAPICS: Mutex<Vec<IoApic>> = Mutex::new(Vec::new());
pub fn register(ioapic: IoApic) {
    IOAPICS.lock().push(ioapic);
}

pub fn enable_irq(gsi: usize, vector: u8, apic_id: u8) {
    log::debug!(
        "Enabling IRQ for GSI {} with vector {} on APIC ID {}",
        gsi,
        vector,
        apic_id
    );

    let ioapics = IOAPICS.lock();
    for ioapic in ioapics.iter() {
        if gsi >= ioapic.gsi_base && gsi < ioapic.gsi_base + (ioapic.max_entires as usize) {
            let local_index = gsi - ioapic.gsi_base;

            let mut entry =
                RedirectionEntry::new(vector, DeliveryMode::FIXED, DestinationMode::PHYSICAL);
            entry.set_destination(apic_id);
            entry.set_masked(false);

            ioapic.write_redirection_entry(local_index, &entry);
            return;
        }
    }

    panic!("No IOAPIC found for GSI {}", gsi);
}

bitflags! {
    pub struct DeliveryMode: u32 {
        const FIXED = 0b000;
        const LOWEST_PRIORITY = 0b001;
        const SMI = 0b010;
        const NMI = 0b100;
        const INIT = 0b101;
        const EXTINT = 0b111;
    }

    pub struct DestinationMode: u32 {
        const PHYSICAL = 0;
        const LOGICAL = 1;
    }
}

pub struct RedirectionEntry(u64);
impl RedirectionEntry {
    pub const fn new(
        vector: u8,
        delivery_mode: DeliveryMode,
        destination_mode: DestinationMode,
    ) -> Self {
        let mut entry = 0;

        entry |= vector as u64;
        entry |= (delivery_mode.bits() as u64) << 8;
        entry |= (destination_mode.bits() as u64) << 11;

        Self(entry)
    }

    pub const fn set_masked(&mut self, masked: bool) {
        if masked {
            self.0 |= 1 << 16;
        } else {
            self.0 &= !(1 << 16);
        }
    }

    pub const fn set_destination(&mut self, apic_id: u8) {
        self.0 &= !(0xFF << 56); // clear the destination field
        self.0 |= (apic_id as u64) << 56; // set the new destination
    }
}

pub struct IoApic {
    base_addr: usize,
    pub gsi_base: usize,
    pub max_entires: u8,
    pub apic_id: u8,
}

impl IoApic {
    pub fn new<A: FrameAllocator>(
        physical_addr: usize,
        gsi_base: usize,
        mapper: &mut Mapper,
        apic_id: u8,
        allocator: &mut A,
    ) -> Self {
        log::info!(
            "Found IO APIC: id = {apic_id}, address = {physical_addr:#010x}, gsi_base = {gsi_base:#010x}",
        );

        let frame = Frame::from_addr(utils::align_down(physical_addr, memory::PAGE_SIZE));

        log::debug!(
            "Mapping IO APIC at physical address {physical_addr:#010x} to virtual address {:#010x}",
            physical_addr,
        );

        mapper.map_to(
            VirtualAddress(physical_addr as _),
            frame,
            EntryFlags::PRESENT
                | EntryFlags::WRITABLE
                | EntryFlags::CACHE_DISABLE
                | EntryFlags::WRITE_THROUGH,
            allocator,
        );

        let mut this = Self {
            base_addr: physical_addr,
            gsi_base,
            max_entires: 0,
            apic_id: 0,
        };

        this.apic_id = apic_id;
        // this.apic_id = (this.read_register(IOAPICID_INDEX) >> 24) as u8;
        this.max_entires = (this.read_register(IOAPICVER_INDEX) >> 16) as u8 + 1;

        this
    }

    pub fn write_redirection_entry(&self, index: usize, entry: &RedirectionEntry) {
        let offset = ioapic_redirection_table_offset(index);
        self.write_register(offset as u8, entry.0 as u32);
        self.write_register((offset + 4) as u8, (entry.0 >> 32) as u32);
    }

    pub fn read_register(&self, index: u8) -> u32 {
        unsafe {
            // put register index in IOREGSEL
            ptr::write_volatile((self.base_addr + IOREGSEL) as *mut u32, index as u32);
            // read from IOWIN
            ptr::read_volatile((self.base_addr + IOWIN) as *const u32)
        }
    }

    pub fn write_register(&self, index: u8, value: u32) {
        unsafe {
            // put register index in IOREGSEL
            ptr::write_volatile((self.base_addr + IOREGSEL) as *mut u32, index as u32);
            // write to IOWIN
            ptr::write_volatile((self.base_addr + IOWIN) as *mut u32, value);
        }
    }
}
