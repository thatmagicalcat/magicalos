mod entry;
mod mapper;
mod table;
pub mod user;

pub use entry::*;
pub use mapper::*;
pub use table::*;

use core::ops::Deref;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalAddress(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualAddress(pub u64);

impl<T> From<T> for PhysicalAddress
where
    T: Into<u64>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl<T> From<T> for VirtualAddress
where
    T: Into<u64>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Deref for PhysicalAddress {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for VirtualAddress {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl VirtualAddress {
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub unsafe fn as_ref<'a, T>(&self) -> &'a T {
        unsafe { &*self.as_ptr() }
    }

    pub unsafe fn as_mut<'a, T>(&self) -> &'a mut T {
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub const fn p4_idx(&self) -> usize {
        (self.0 as usize >> 39) & 0o777
    }

    pub const fn p3_idx(&self) -> usize {
        (self.0 as usize >> 30) & 0o777
    }

    pub const fn p2_idx(&self) -> usize {
        (self.0 as usize >> 21) & 0o777
    }

    pub const fn p1_idx(&self) -> usize {
        (self.0 as usize >> 12) & 0o777
    }
}

impl PhysicalAddress {
    // pub const fn new(
    //     p4_idx: usize,
    //     p3_idx: usize,
    //     p2_idx: usize,
    //     p1_idx: usize,
    //     offset: usize,
    // ) -> Self {
    //     let mut addr = ((p4_idx as u64 & 0o777) << 39)
    //         | ((p3_idx as u64 & 0o777) << 30)
    //         | ((p2_idx as u64 & 0o777) << 21)
    //         | ((p1_idx as u64 & 0o777) << 12)
    //         | (offset as u64 & 0xfff);
    //
    //     if addr & 1 << 47 != 0 {
    //         addr |= 0xffff_0000_0000_0000;
    //     }
    //
    //     Self(addr)
    // }

    pub fn to_virtual(&self, hhdm_offest: u64) -> VirtualAddress {
        (self.0 + hhdm_offest).into()
    }
}
