//! source: https://wiki.osdev.org/HPET

use core::time::Duration;

use spin::Once;

use crate::{
    memory::{
        self, Frame, FrameAllocator,
        paging::{Mapper, PageTableEntryFlags, VirtualAddress},
    },
    utils,
};

pub static HPET: Once<Hpet> = Once::new();

const HPET_CAPABILITES: usize = 0x00;
const HPET_CONFIG: usize = 0x10;
const HPET_COUNTER: usize = 0xF0;

pub struct Hpet {
    base_address: usize,

    /// in femtoseconds (10^-15 seconds)
    pub time_period: usize,
}

impl Hpet {
    pub fn new<A: FrameAllocator>(
        base_addr: usize,
        mapper: &mut Mapper,
        allocator: &mut A,
    ) -> Self {
        log::info!(
            "Mapping HPET at physical address {base_addr:#010x} to virtual address {:#010x}",
            base_addr
        );

        // TODO: Map HPET to higher half address
        let frame = Frame::from_addr(utils::align_down(base_addr, memory::PAGE_SIZE));
        mapper.map_to(
            VirtualAddress(base_addr as _),
            frame,
            PageTableEntryFlags::PRESENT
                | PageTableEntryFlags::WRITABLE
                | PageTableEntryFlags::CACHE_DISABLE
                | PageTableEntryFlags::WRITE_THROUGH,
            allocator,
        );

        let mut this = Self {
            base_address: base_addr,
            time_period: 0,
        };

        this.time_period = this.read_capabilities() >> 32;
        this.enable();

        this
    }

    pub fn busy_wait(&self, duration: Duration) {
        let ticks = duration.as_nanos() * 1_000_000 / self.time_period as u128;
        let start = self.read_counter() as u128;

        while self.read_counter() as u128 - start < ticks {
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    fn read_capabilities(&self) -> usize {
        unsafe { (self.base_address as *const usize).read_volatile() }
    }

    #[inline(always)]
    fn read_config(&self) -> usize {
        unsafe { ((self.base_address + HPET_CONFIG) as *const usize).read_volatile() }
    }

    #[inline(always)]
    fn write_config(&self, value: usize) {
        unsafe { ((self.base_address + HPET_CONFIG) as *mut usize).write_volatile(value) }
    }

    #[inline(always)]
    pub fn read_counter(&self) -> usize {
        unsafe { ((self.base_address + HPET_COUNTER) as *const usize).read_volatile() }
    }

    pub fn uptime_nanos(&self) -> u64 {
        let counter = self.read_counter() as u128;

        // time_period is in femtoseconds (10^-15)
        // femtoseconds to nanoseconds = divide by 1_000_000
        ((counter * self.time_period as u128) / 1_000_000) as u64
    }

    fn enable(&self) {
        let mut config = self.read_config();
        config |= 1; // set bit 0 to enable HPET
        self.write_config(config);
    }
}
