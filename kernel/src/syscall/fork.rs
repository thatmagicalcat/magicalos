use alloc::boxed::Box;

use crate::{
    arch, memory,
    scheduler::{self, Task, TaskStack},
};

#[repr(C)]
pub struct CalleeSaved {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
}


/// the contents were deep copied from the parent frame
/// but we want to enqueue this task directly to the
/// scheduler, so we need to remove the information
/// stored by the syscall_handler
#[unsafe(naked)]
extern "C" fn fork_ret() {
    core::arch::naked_asm! {
        "xor rax, rax", // return 0 for child
        "cli",
        "pop rcx",      // user stack pointer
        "mov rsp, rcx", // switch back to user stack
        "swapgs",       // restore user GS base
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "sysretq"
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub(crate) extern "C" fn sys_fork() -> u64 {
    core::arch::naked_asm! {
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, rsp", // pass the pointer to CalleeSaved as the first argument
        "call {do_sys_fork}",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "ret",
        do_sys_fork = sym do_sys_fork,
    }
}

#[unsafe(no_mangle)]
extern "C" fn do_sys_fork(regs: *const CalleeSaved) -> u64 {
    log::trace!("Enter sys_fork");

    scheduler::with_current_task(|parent| {
        let child_id = scheduler::new_task_id();
        let child_p4_table = memory::paging::user::fork_current_page_table();

        let mut child_task = Task {
            id: child_id,
            status: scheduler::TaskStatus::Ready,
            last_stack_ptr: 0,
            stack: Box::new(TaskStack::new()),
            root_page_table: child_p4_table,
            fd_map: parent.fd_map.clone(),
            vmspace: parent.vmspace.clone(),
            fpu_state: parent.fpu_state.clone(),
            cfg: parent.cfg.clone(),
        };

        let mut child_sp = child_task.stack.top().0 as *mut u64;
        let parent_sp = parent.stack.top().0 as *const u64;

        // `syscall_handler` pushed `rcx` (user rsp) right after `rdgsbase`.
        // this is exactly located at the top of the parent's kernel stack
        let user_rsp = unsafe { *parent_sp.offset(-1) };

        let fs_base = unsafe { arch::msr::rdmsr(arch::msr::IA32_FS_BASE) };
        // let kernel_gs_base = unsafe { arch::msr::rdmsr(arch::msr::IA32_GS_BASE) };
        let kernel_gs_base = child_task.stack.top().0;

        unsafe {
            let saved = &*regs;
            child_sp = child_sp.offset(-1); *child_sp = user_rsp; 
            child_sp = child_sp.offset(-1); *child_sp = fork_ret as *const () as u64; 
            
            // replicate `restore_context!` layout:
            child_sp = child_sp.offset(-1); *child_sp = 0x202; // popfq
            child_sp = child_sp.offset(-1); *child_sp = 0;     // rax
            child_sp = child_sp.offset(-1); *child_sp = 0;     // rcx
            child_sp = child_sp.offset(-1); *child_sp = 0;     // rdx
            child_sp = child_sp.offset(-1); *child_sp = saved.rbx; // rbx
            child_sp = child_sp.offset(-1); *child_sp = 0;     // dummy for 'add rsp, 8'
            child_sp = child_sp.offset(-1); *child_sp = saved.rbp; // rbp
            child_sp = child_sp.offset(-1); *child_sp = 0;     // rsi
            child_sp = child_sp.offset(-1); *child_sp = 0;     // rdi
            child_sp = child_sp.offset(-1); *child_sp = 0;     // r8
            child_sp = child_sp.offset(-1); *child_sp = 0;     // r9
            child_sp = child_sp.offset(-1); *child_sp = 0;     // r10
            child_sp = child_sp.offset(-1); *child_sp = 0;     // r11
            child_sp = child_sp.offset(-1); *child_sp = saved.r12; // r12
            child_sp = child_sp.offset(-1); *child_sp = saved.r13; // r13
            child_sp = child_sp.offset(-1); *child_sp = saved.r14; // r14
            child_sp = child_sp.offset(-1); *child_sp = saved.r15; // r15
            child_sp = child_sp.offset(-1); *child_sp = fs_base;
            child_sp = child_sp.offset(-1); *child_sp = kernel_gs_base;
        }

        child_task.last_stack_ptr = child_sp as usize;
        scheduler::enqueue_task(child_task);

        // return the PID of child process
        crate::dbg!(child_id.into())
    })
}
