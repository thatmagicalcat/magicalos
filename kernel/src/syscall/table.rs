use crate::syscall::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(usize)]
pub enum Syscall {
    Exit = 0,
    Read,
    Write,
    MemMap,
    ArchPrctl,
    Open,
    Close,
    Clock,

    /// Not a valid syscall
    NumSyscalls,
}

#[repr(align(64))]
#[repr(C)]
pub(crate) struct SyscallTable {
    handle: [*const usize; Syscall::NumSyscalls as usize],
}

impl SyscallTable {
    pub const fn new() -> Self {
        Self {
            handle: [
                /* 0 */ exit::sys_exit as _,
                /* 1 */ read::sys_read as _,
                /* 2 */ write::sys_write as _,
                /* 3 */ mmap::sys_mmap as _,
                /* 4 */ arch_prctl::sys_arch_prctl as _,
                /* 5 */ open::sys_open as _,
                /* 6 */ close::sys_close as _,
                /* 7 */ clock::sys_clock as _,
            ],
        }
    }
}

/// SAFETY: trust me bro
unsafe impl Send for SyscallTable {}
unsafe impl Sync for SyscallTable {}

#[unsafe(no_mangle)]
pub(crate) static SYSCALL_TABLE: SyscallTable = SyscallTable::new();
