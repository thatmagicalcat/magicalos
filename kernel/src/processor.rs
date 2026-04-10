use core::arch::{asm, naked_asm};

use raw_cpuid::CpuId;

use crate::{kernel::USER_ENTRY, syscall::SYSCALL_TABLE, utils};

pub fn init() {
    let cpuid = CpuId::new();

    let has_fsgsbase = match cpuid.get_extended_feature_info() {
        Some(efinfo) => efinfo.has_fsgsbase(),
        None => false,
    };

    if has_fsgsbase {
        let mut cr4 = utils::read_cr4();
        cr4 |= 1 << 16;
        utils::write_cr4(cr4);
    } else {
        panic!("ThatMagicalOS requires the CPU feature FSGSBASE");
    }
}

pub unsafe fn jump_to_user_fn(entry_point: extern "C" fn()) -> ! {
    let ds = 0x1b_usize; // GDT Index 3, Ring 3
    let cs = 0x23_usize; // GDT Index 4, Ring 3

    unsafe {
        __jump_to_user_land(
            ds,
            USER_ENTRY.0 as usize + 0x400000_usize,
            cs,
            USER_ENTRY.0 as usize | entry_point as usize & 0xFFFusize,
        )
    }

    // unsafe {
    //     asm! {
    //         "push {0}",
    //         "push {1}",
    //         "add qword ptr [rsp], 16",
    //         "pushf",
    //         "push {2}",
    //         "push {3}",
    //         "iretq",
    //         in(reg) ds,
    //         in(reg) stack_ptr,
    //         in(reg) cs,
    //         in(reg) entry_point as usize,
    //         options(nostack)
    //     }
    // }
    //
    // loop {
    //     unsafe { asm!("hlt") }
    // }
}

#[unsafe(naked)]
unsafe extern "C" fn __jump_to_user_land(ds: usize, stack: usize, cs: usize, entry: usize) -> ! {
    naked_asm! {
        "swapgs",
        "push rdi",
        "push rsi",
        "push 0x202", // pushf
        "push rdx",
        "push rcx",
        "iretq",
    }
}

/// Helper function to save and to restore the register states
/// during a system call. `rax` is the system call identifier.
/// The identifier is used to determine the address of the function,
/// which implements the system call.
#[unsafe(naked)]
pub(crate) extern "C" fn syscall_handler() {
    naked_asm! {
        // save context, see x86_64 ABI
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        // switch to kernel stack
        "swapgs",
        "mov rcx, rsp",
        "rdgsbase rsp",
        "push rcx",
        // copy 4th argument to rcx to adhere x86_64 ABI
        "mov rcx, r10",
        "sti",
        "call [{sys_table}+8*rax]",
        // restore context, see x86_64 ABI
        "cli",
        // switch to user stack
        "pop rcx",
        "mov rsp, rcx",
        "swapgs",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "sysretq",
        sys_table = sym SYSCALL_TABLE,
    };
}

#[macro_export]
macro_rules! syscall {
    ($arg0:expr) => {
        $crate::processor::syscall0($arg0 as _)
    };

    ($arg0:expr, $arg1:expr) => {
        $crate::processor::syscall1($arg0 as _, $arg1 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr) => {
        $crate::processor::syscall2($arg0 as _, $arg1 as _, $arg2 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        $crate::processor::syscall3($arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        arch::x86::syscall4($arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
        $crate::processor::syscall5(
            $arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _, $arg5 as _,
        )
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {
        $crate::processor::syscall6(
            $arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _, $arg5 as _, $arg6 as _,
        )
    }; // ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr) => {
       //     $crate::processor::syscall7(
       //         $arg0 as _,
       //         $arg1 as _,
       //         $arg2 as _,
       //         $arg3 as _,
       //         $arg4 as _,
       //         $arg5 as _,
       //         $arg6 as _,
       //         $arg7 as _,
       //     )
       // };
}

#[inline(always)]
pub fn syscall0(arg0: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall1(arg0: u64, arg1: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall2(arg0: u64, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall3(arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall4(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall5(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}

#[inline(always)]
pub fn syscall6(
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let ret: u64;

    unsafe {
        asm! {
            "syscall",

            inlateout("rax") arg0 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            in("r9") arg6,

            lateout("rcx") _,
            lateout("r11") _,

            options(preserves_flags, nostack)
        }
    }

    ret
}
