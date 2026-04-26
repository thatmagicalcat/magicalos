use crate::syscall::{
    arch_prctl::sys_arch_prctl, close::sys_close, exit::sys_exit, mmap::sys_mmap, open::sys_open,
    read::sys_read, write::sys_write,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(usize)]
pub enum Syscall {
    Exit = 0,
    Read = 1,
    Write = 2,
    MemMap = 3,
    ArchPrctl = 4,
    Open = 5,
    Close = 6,

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
                sys_exit as _,
                sys_read as _,
                sys_write as _,
                sys_mmap as _,
                sys_arch_prctl as _,
                sys_open as _,
                sys_close as _,
            ],
        }
    }
}

/// SAFETY: trust me bro
unsafe impl Send for SyscallTable {}
unsafe impl Sync for SyscallTable {}

#[unsafe(no_mangle)]
pub(crate) static SYSCALL_TABLE: SyscallTable = SyscallTable::new();
