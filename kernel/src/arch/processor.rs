use core::arch::{asm, naked_asm};

use raw_cpuid::CpuId;

use crate::{syscall::SYSCALL_TABLE, utils};

#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub fsgsbase: bool,
    pub sse: bool,
}

pub fn detect_cpu_features() -> CpuFeatures {
    let cpuid = CpuId::new();

    let fsgsbase = cpuid
        .get_extended_feature_info()
        .is_some_and(|f| f.has_fsgsbase());

    let sse = cpuid.get_feature_info().is_some_and(|f| f.has_sse());

    CpuFeatures { fsgsbase, sse }
}

pub fn init() {
    let features = detect_cpu_features();

    enable_fsgsbase(features);
    enable_fpu(features);
}

fn enable_fsgsbase(features: CpuFeatures) {
    if !features.fsgsbase {
        panic!("MagicalOS requires FSGSBASE");
    }

    let mut cr4 = utils::read_cr4();
    cr4 |= 1 << 16; // FSGSBASE
    utils::write_cr4(cr4);

    log::info!("FSGSBASE enabled");
}

pub fn enable_fpu(features: CpuFeatures) {
    if !features.sse {
        log::error!("SSE not supported — continuing anyway (this is sketchy)");
        return;
    }

    log::info!("Enabling SSE and FPU");

    let mut cr0: usize;

    unsafe { asm!("mov {}, cr0", out(reg) cr0) };
    cr0 &= !(1 << 2); // clear EM
    cr0 |= (1 << 1) | (1 << 5); // MP and NE
    unsafe { asm!("mov cr0, {}", in(reg) cr0) };

    let mut cr4: usize;
    unsafe { asm!("mov {}, cr4", out(reg) cr4) };
    cr4 |= (1 << 9) | (1 << 10); // OSFXSR and OSXMMEXCPT
    unsafe { asm!("mov cr4, {}", in(reg) cr4) };
}

pub unsafe fn jump_to_user_fn(entry_point: usize, stack_ptr: usize) -> ! {
    let ds = 0x1b_usize; // GDT Index 3, Ring 3
    let cs = 0x23_usize; // GDT Index 4, Ring 3

    unsafe {
        __jump_to_user_land(
            ds,
            // an arbitrary user stack, which is used by pagefault_handler for demand paging!
            stack_ptr,
            cs,
            entry_point,
            // USER_ENTRY.0 as usize | entry_point & 0xFFFusize,
        )
    }
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

#[inline(always)]
pub fn rdtscp() -> u64 {
    let mut aux: u32 = 0;
    unsafe { core::arch::x86_64::__rdtscp(&mut aux) }
}

/// Helper function to save and to restore the register states
/// during a system call. `rax` is the system call identifier.
/// The identifier is used to determine the address of the function,
/// which implements the system call.
#[unsafe(naked)]
pub(crate) extern "C" fn syscall_handler() {
    naked_asm! {
        // save context
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
        // copy 4th argument to rcx
        "mov rcx, r10",
        "sti",
        "call [{sys_table}+8*rax]",
        "cli",
        // switch to user stack
        "pop rcx",
        "mov rsp, rcx",
        // restore context
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
        $crate::arch::processor::syscall0($arg0 as _)
    };

    ($arg0:expr, $arg1:expr) => {
        $crate::arch::processor::syscall1($arg0 as _, $arg1 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr) => {
        $crate::arch::processor::syscall2($arg0 as _, $arg1 as _, $arg2 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        $crate::arch::processor::syscall3($arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        arch::x86::syscall4($arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _)
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
        $crate::arch::processor::syscall5(
            $arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _, $arg5 as _,
        )
    };

    ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {
        $crate::arch::processor::syscall6(
            $arg0 as _, $arg1 as _, $arg2 as _, $arg3 as _, $arg4 as _, $arg5 as _, $arg6 as _,
        )
    }; // ($arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr) => {
       //     $crate::arch::processor::syscall7(
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
