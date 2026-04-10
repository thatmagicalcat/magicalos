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

