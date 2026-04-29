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
    Sleep,
    Seek,
    Mkdir,

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
                /* 8 */ sleep::sys_sleep as _,
                /* 9 */ seek::sys_seek as _,
                /* 10 */ mkdir::sys_mkdir as _,
            ],
        }
    }
}

/// SAFETY: trust me bro
unsafe impl Send for SyscallTable {}
unsafe impl Sync for SyscallTable {}

#[unsafe(no_mangle)]
pub(crate) static SYSCALL_TABLE: SyscallTable = SyscallTable::new();
