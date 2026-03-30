use core::{
    alloc::Layout,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::boxed::Box;

use crate::gdt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Idle,
    Sleeping(u64), // target timestamp (in nanoseconds)
    Blocked, // for future use, e.g. waiting for I/O
    Terminated,
}

#[derive(Debug)]
pub struct Thread {
    pub id: u64,
    pub stack_ptr: *mut u8,
    pub stack_layout: Layout,
    pub state: ThreadState,
    pub rsp: u64,
}

// SAFETY: send because it can be safely sent between threads
// because *mut u8 is only used for the thread's own stack and is not shared between threads
unsafe impl Send for Thread {}

#[derive(Debug)]
pub struct Context {
    pub rsp: u64,
}

impl Thread {
    pub fn new(entry_point: fn()) -> Box<Self> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        let stack_layout = Layout::from_size_align(4096, 16).unwrap();
        let stack_ptr = unsafe { alloc::alloc::alloc(stack_layout) };

        let stack_top = stack_ptr as u64 + 4096;
        let mut rsp = stack_top as *mut u64;

        unsafe {
            // 1. hardware IRETQ frame

            rsp = rsp.sub(1);
            ptr::write(rsp, gdt::KERNEL_DATA_SELECTOR as _); // ss

            rsp = rsp.sub(1);
            ptr::write(rsp, stack_top); // rsp

            rsp = rsp.sub(1);
            ptr::write(rsp, 0x202); // rflags

            rsp = rsp.sub(1);
            ptr::write(rsp, gdt::KERNEL_CODE_SELECTOR as _); // cs

            rsp = rsp.sub(1);
            ptr::write(rsp, entry_point as usize as _); // rip

            // reserve space for 15 general purpose registers that will be pushed/popped during
            // context switch
            rsp = rsp.sub(15);
        }

        Box::new(Self {
            id,
            stack_ptr,
            stack_layout,
            rsp: rsp as u64,
            state: ThreadState::Ready,
        })
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        unsafe { alloc::alloc::dealloc(self.stack_ptr, self.stack_layout) };
    }
}

// #[unsafe(naked)]
// unsafe extern "C" fn ctx_switch(old_stack_ptr: *mut u64, new_stak_ptr: *const u64) {
//     core::arch::naked_asm! {
//         // at this point, the return address should already be on the stack
//
//         // save current registers
//         "push r15",
//         "push r14",
//         "push r13",
//         "push r12",
//         "push r11",
//         "push r10",
//         "push r9",
//         "push r8",
//         "push rbp",
//         "push rdi",
//         "push rsi",
//         "push rdx",
//         "push rcx",
//         "push rbx",
//         "push rax",
//
//         // save the stack pointer to the old context
//         "mov [rdi], rsp",
//
//         // switch to the new context
//         "mov rsp, [rsi]",
//
//         // load the registers from the new context
//         "pop rax",
//         "pop rbx",
//         "pop rcx",
//         "pop rdx",
//         "pop rsi",
//         "pop rdi",
//         "pop rbp",
//         "pop r8",
//         "pop r9",
//         "pop r10",
//         "pop r11",
//         "pop r12",
//         "pop r13",
//         "pop r14",
//         "pop r15",
//
//         // we already pushed the return address when creating the thread
//         // so we can just return to the new thread's entry point
//         "iret",
//     }
// }
