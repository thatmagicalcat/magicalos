use core::ptr::NonNull;

use acpi::{Handler, PciAddress, PhysicalMapping};

use crate::{kernel, limine_requests::HHDM_REQUEST};

use super::port::Port;

#[derive(Clone)]
pub struct KernelAcpiHandler;

impl Handler for KernelAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let hhdm_offset = kernel::get_hhdm_offset();
        let virtual_address = physical_address + hhdm_offset;

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(virtual_address as *mut T)
                .expect("Failed to create NonNull pointer for mapped region"),
            region_length: size,
            mapped_length: size,
            handler: self.clone(),
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}

    fn read_u8(&self, address: usize) -> u8 {
        unsafe { core::ptr::read_unaligned(address as *const u8) }
    }
    fn read_u16(&self, address: usize) -> u16 {
        unsafe { core::ptr::read_unaligned(address as *const u16) }
    }
    fn read_u32(&self, address: usize) -> u32 {
        unsafe { core::ptr::read_unaligned(address as *const u32) }
    }
    fn read_u64(&self, address: usize) -> u64 {
        unsafe { core::ptr::read_unaligned(address as *const u64) }
    }
    fn write_u8(&self, address: usize, value: u8) {
        unsafe { core::ptr::write_unaligned(address as *mut u8, value) }
    }
    fn write_u16(&self, address: usize, value: u16) {
        unsafe { core::ptr::write_unaligned(address as *mut u16, value) }
    }
    fn write_u32(&self, address: usize, value: u32) {
        unsafe { core::ptr::write_unaligned(address as *mut u32, value) }
    }
    fn write_u64(&self, address: usize, value: u64) {
        unsafe { core::ptr::write_unaligned(address as *mut u64, value) }
    }
    fn read_io_u8(&self, port: u16) -> u8 {
        unsafe { u8::read_from_port(port) }
    }
    fn read_io_u16(&self, port: u16) -> u16 {
        unsafe { u16::read_from_port(port) }
    }
    fn read_io_u32(&self, port: u16) -> u32 {
        unsafe { u32::read_from_port(port) }
    }
    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe { u8::write_to_port(port, value) }
    }
    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe { u16::write_to_port(port, value) }
    }
    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe { u32::write_to_port(port, value) }
    }
    fn read_pci_u8(&self, _address: PciAddress, _offset: u16) -> u8 {
        unimplemented!("PCI unimplemented")
    }
    fn read_pci_u16(&self, _address: PciAddress, _offset: u16) -> u16 {
        unimplemented!("PCI unimplemented")
    }
    fn read_pci_u32(&self, _address: PciAddress, _offset: u16) -> u32 {
        unimplemented!("PCI unimplemented")
    }
    fn write_pci_u8(&self, _address: PciAddress, _offset: u16, _value: u8) {
        unimplemented!("PCI unimplemented")
    }
    fn write_pci_u16(&self, _address: PciAddress, _offset: u16, _value: u16) {
        unimplemented!("PCI unimplemented")
    }
    fn write_pci_u32(&self, _address: PciAddress, _offset: u16, _value: u32) {
        unimplemented!("PCI unimplemented")
    }
    fn nanos_since_boot(&self) -> u64 {
        unimplemented!("Timers unimplemented")
    }
    fn stall(&self, _microseconds: u64) {
        unimplemented!("Timers unimplemented")
    }
    fn sleep(&self, _milliseconds: u64) {
        unimplemented!("Timers unimplemented")
    }
    fn create_mutex(&self) -> acpi::Handle {
        unimplemented!("Mutexes unimplemented")
    }
    fn acquire(&self, _mutex: acpi::Handle, _timeout: u16) -> Result<(), acpi::aml::AmlError> {
        unimplemented!("Mutexes unimplemented")
    }
    fn release(&self, _mutex: acpi::Handle) {
        unimplemented!("Mutexes unimplemented")
    }
}
