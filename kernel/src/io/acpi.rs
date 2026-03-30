use alloc::sync::Arc;
use core::ptr::NonNull;

use acpi::{Handler, PciAddress, PhysicalMapping};

use spin::Mutex;

use super::port::Port;
use crate::{
    memory::{
        self, Frame, PAGE_SIZE,
        paging::{ActivePageTable, EntryFlags, VirtualAddress},
    },
    utils,
};

#[derive(Clone)]
pub struct KernelAcpiHandler<const N: usize> {
    allocator: Arc<Mutex<memory::TinyAllocator<N>>>,
}

impl<const N: usize> KernelAcpiHandler<N> {
    pub const fn new(allocator: Arc<Mutex<memory::TinyAllocator<N>>>) -> Self {
        Self { allocator }
    }

    pub fn into_allocator(self) -> memory::TinyAllocator<N> {
        Arc::try_unwrap(self.allocator)
            .expect("Failed to unwrap allocator Arc")
            .into_inner()
    }
}

impl<const N: usize> Handler for KernelAcpiHandler<N> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let mut active_page_table = ActivePageTable::new();
        let mut allocator = self.allocator.lock();

        // page-align the physical address
        let start_frame = Frame::from_addr(utils::align_down(physical_address, memory::PAGE_SIZE));
        let end_frame =
            Frame::from_addr(utils::align_up(physical_address + size, memory::PAGE_SIZE));

        for frame in start_frame.0..end_frame.0 {
            // identity map the kernel section
            let page = VirtualAddress((frame * PAGE_SIZE) as _);
            active_page_table.mapper_mut().map_if_unmapped(
                page,
                Frame(frame),
                EntryFlags::WRITABLE | EntryFlags::PRESENT,
                &mut *allocator,
            );
        }

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(physical_address as *mut T)
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
