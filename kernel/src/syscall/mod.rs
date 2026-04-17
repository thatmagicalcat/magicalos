mod table;

mod exit;
mod write;
mod mmap;

pub use table::*;

use crate::{arch::msr::*, arch::processor::syscall_handler};

const EFER_SCE: u64 = 1 << 0;
const EFER_LMA: u64 = 1 << 10;

#[rustfmt::skip]
pub fn init() {
    unsafe {
        wrmsr(IA32_EFER, rdmsr(IA32_EFER) | EFER_LMA | EFER_SCE);
        wrmsr(IA32_STAR, (0x13u64 << 48) | (0x08u64 << 32));
        wrmsr(IA32_LSTAR, (syscall_handler as *const () as usize).try_into().unwrap());
        wrmsr(IA32_FMASK, 1 << 9); // clear IF flag during system call
    }
}
