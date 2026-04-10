use crate::syscall::{exit::sys_exit, write::sys_write};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(usize)]
pub enum Syscall {
    Exit = 0,
    Write = 1,

    /// Not a valid syscall
    NumSyscalls = 2,
}

#[repr(align(64))]
#[repr(C)]
pub(crate) struct SyscallTable {
    handle: [*const usize; Syscall::NumSyscalls as usize],
}

impl SyscallTable {
    pub const fn new() -> Self {
        Self {
            handle: [sys_exit as _, sys_write as _],
        }
    }
}

/// SAFETY: trust me bro
unsafe impl Send for SyscallTable {}
unsafe impl Sync for SyscallTable {}

#[unsafe(no_mangle)]
pub(crate) static SYSCALL_TABLE: SyscallTable = SyscallTable::new();
