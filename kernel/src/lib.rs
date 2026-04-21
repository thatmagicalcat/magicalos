#![no_std]
#![cfg_attr(all(test, target_os = "none"), no_main)]
#![warn(clippy::missing_const_for_fn)]
#![feature(custom_test_frameworks)]
#![feature(linked_list_cursors)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

pub mod arch;
pub mod async_rt;
pub mod auxvec;
pub mod bus;
pub mod drivers;
pub mod elf;
pub mod errno;
pub mod fd;
pub mod fs;
pub mod io;
pub mod kernel;
pub mod limine_requests;
pub mod macros;
pub mod memory;
pub mod scheduler;
pub mod synch;
pub mod syscall;
pub mod testing;
pub mod utils;

use alloc::{sync::Arc, vec::Vec};
use kernel::USER_ENTRY;

use elf::*;
use io::Read;

use crate::{
    arch::processor,
    kernel::{USER_STACK_BOTTOM, USER_STACK_TOP},
    memory::paging::{PageTableEntryFlags, VirtualAddress},
};

#[rustfmt::skip]
pub(crate) const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info" )] { log::LevelFilter::Info  }
    #[cfg(log_level = "warn" )] { log::LevelFilter::Warn  }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

pub fn kentry() {
    kernel::init();

    scheduler::spawn(
        || load_elf(c"/home/thatmagicalcat/main.elf".as_ptr()),
        scheduler::NORMAL_PRIORITY,
    )
    .unwrap();

    // scheduler::spawn(
    //     || loop {
    //         println!("ello!");
    //         arch::hpet::HPET
    //             .get()
    //             .unwrap()
    //             .busy_wait(core::time::Duration::from_millis(100));
    //     },
    //     scheduler::NORMAL_PRIORITY,
    // )
    // .unwrap();

    scheduler::spawn(
        || {
            let mut async_rt = async_rt::Executor::new();
            async_rt.spawn(drivers::keyboard::handle_keypresses());
            async_rt.run();
        },
        scheduler::REALTIME_PRIORITY,
    )
    .unwrap();

    scheduler::reschedule();

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "C" fn load_elf(path: *const i8) {
    utils::write_cr3(*memory::paging::user::create_page_table() as _);

    let mut elf_data = Vec::new();
    let mut file = fs::File::open(unsafe { core::ffi::CStr::from_ptr(path).to_str().unwrap() })
        .expect("Failed to open user ELF file");
    file.read_to_end(&mut elf_data)
        .expect("Failed to read user ELF file");

    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        panic!("Buffer too small");
    }

    let header = unsafe { core::ptr::read_unaligned(elf_data.as_ptr() as *const Elf64Header) };

    assert!(header.ident.magic == ELF_OBJECT_MAGIC);

    let type_ = header.type_;
    let base_address: usize = if type_ == ObjectFileType::Dynamic {
        log::debug!("Dynamic ELF detected, using USER_ENTRY as base address");
        USER_ENTRY.0 as _
    } else {
        0
    };

    let entry = base_address + header.entry as usize;
    let ph_off = header.program_header_table_offset as usize;
    let ph_num = header.program_header_table_num_entires as usize;
    let ph_ent = header.program_header_table_entry_size as usize;

    let elf_arc = Arc::new(elf_data);

    scheduler::with_current_task(|task| {
        // prevent mmaping the stack
        task.vmspace
            .insert(
                USER_STACK_BOTTOM.0 as _,
                USER_STACK_TOP.0 as _,
                PageTableEntryFlags::USER_ACCESSIBLE
                    | PageTableEntryFlags::WRITABLE
                    | PageTableEntryFlags::NO_EXECUTE,
                memory::MappingType::Anonymous,
            )
            .unwrap();

        for i in 0..ph_num {
            let offset = ph_off + i * ph_ent;
            let phdr: Elf64ProgramHeader = unsafe {
                core::ptr::read_unaligned(elf_arc.as_ptr().add(offset) as *const Elf64ProgramHeader)
            };

            if phdr.type_ != Elf64ProgramHeaderType::Load {
                log::warn!("PHDR type: {:?} is not supported yet", phdr.type_);
                continue;
            }

            let vaddr = base_address + phdr.virual_address as usize;
            let mem_size = phdr.mem_size as usize;
            let file_size = phdr.file_size as usize;
            let file_offset = phdr.offset as usize;

            let misalign = vaddr % memory::PAGE_SIZE;

            let start_page = vaddr - misalign;
            let end_page = utils::align_up(vaddr + mem_size, memory::PAGE_SIZE);

            let aligned_file_offset = file_offset - misalign;
            let aligned_file_size = file_size + misalign;

            let mut flags = PageTableEntryFlags::USER_ACCESSIBLE;

            if phdr.flags & Elf64ProgramHeaderFlag::Write as u32 != 0 {
                flags |= PageTableEntryFlags::WRITABLE;
            }

            if phdr.flags & Elf64ProgramHeaderFlag::Execute as u32 == 0 {
                flags |= PageTableEntryFlags::NO_EXECUTE;
            }

            task.vmspace
                .insert(
                    start_page,
                    end_page,
                    flags,
                    memory::MappingType::Elf {
                        data: Arc::clone(&elf_arc),
                        file_offset: aligned_file_offset,
                        file_size: aligned_file_size,
                    },
                )
                .expect("Failed to insert ELF VMA");
        }
    });

    /*
     * usr_stack_top somewhere around here
     *
     * High addresses
     * +----------------------------------------+
     * |         INFORMATION BLOCK              |
     * |      (actual raw string bytes)         |
     * +----------------------------------------+
     * | "PATH=/bin\0" <- e.g. of envp string 1 |
     * | "USER=cat"    <- e.g. of envp string 0 |
     * | "./usr.elf\0" <- e.g. of argv string 0 |
     * +----------------------------------------+
     * | 16 bytes of random data (AT_RANDOM)    |
     * +----------------------------------------+
     * | 0 to 15 bytes of padding               |
     * +----------------------------------------+
     * | auxv AT_NULL <- end of auxv            |
     * | auxv value                             |
     * | auxv key (AT_RANDOM)                   |
     * | ...                                    |
     * | auxv value (e.g. 4096)                 |
     * | auxv key (AT_PAGESZ) <- start of auxv  |
     * +----------------------------------------+
     * | NULL <- end of envp array              |
     * | envp pointers                          |
     * +----------------------------------------+
     * | NULL <- end of argv array              |
     * | argv pointers                          |
     * +----------------------------------------+
     * | argc <- argument count                 |
     * +----------------------------------------+
     * Low addresses (<- final rsp)
     * */

    // map the very first page of the user stack so that we can write to it
    let usr_stack_top = USER_STACK_TOP.0 as usize;
    let stack_page_addr = usr_stack_top - memory::PAGE_SIZE;
    let hhdm_offset = kernel::get_hhdm_offset();
    let frame = memory::allocate_frame().expect("oom");
    let hhdm_ptr = (frame.start_address() + hhdm_offset) as *mut u8;

    // zero out everything
    unsafe { core::ptr::write_bytes(hhdm_ptr, 0, memory::PAGE_SIZE) };

    memory::paging::PageTable::active().mapper_mut().map_to(
        VirtualAddress(stack_page_addr as _),
        frame,
        PageTableEntryFlags::USER_ACCESSIBLE
            | PageTableEntryFlags::WRITABLE
            | PageTableEntryFlags::NO_EXECUTE,
        &mut *memory::lock_global_frame_allocator(),
    );

    let mut current_rsp = usr_stack_top;
    let rsp = &mut current_rsp;

    let push_bytes = |bytes: &[u8], rsp: &mut usize| {
        *rsp -= bytes.len();
        let offset = *rsp - stack_page_addr;
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), hhdm_ptr.add(offset), bytes.len());
        }

        // user-space virtual address of pushed bytes
        *rsp
    };

    let push = |v: u64, rsp: &mut usize| {
        *rsp -= 8;
        let offset = *rsp - stack_page_addr;
        unsafe { core::ptr::write_unaligned(hhdm_ptr.add(offset).cast(), v) };
    };

    // 16 bytes of random data
    // TODO: generate this value using a PRNG
    let entropy: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF,
    ];
    let at_random_ptr = push_bytes(&entropy, rsp);

    let env_str = b"FOO=BAR\0";
    let envp0_ptr = push_bytes(env_str, rsp);

    let path_bytes = unsafe { core::ffi::CStr::from_ptr(path).to_bytes_with_nul() };
    let argv0_ptr = push_bytes(path_bytes, rsp);

    // NOTE: make sure to update this
    // argc (1), argv (2), envp (2) + auxv (16) = 21 slots
    let final_rsp_unaligned = *rsp - (21 * 8);
    let final_rsp_aligned = utils::align_down(final_rsp_unaligned, 16);
    let padding = final_rsp_unaligned - final_rsp_aligned;
    *rsp -= padding;

    // --------- push ---------
    push(0, rsp);
    push(auxvec::AT_NULL, rsp);

    push(at_random_ptr as _, rsp);
    push(auxvec::AT_RANDOM, rsp);

    push(entry as _, rsp);
    push(auxvec::AT_ENTRY, rsp);

    push(base_address as _, rsp);
    push(auxvec::AT_BASE, rsp);

    push(memory::PAGE_SIZE as _, rsp);
    push(auxvec::AT_PAGESZ, rsp);

    push(ph_num as _, rsp);
    push(auxvec::AT_PHNUM, rsp);

    push(ph_ent as _, rsp);
    push(auxvec::AT_PHENT, rsp);

    push((ph_off + base_address) as _, rsp);
    push(auxvec::AT_PHDR, rsp);

    // push envp
    push(0, rsp); // NULL
    push(envp0_ptr as _, rsp);

    // push argv
    push(0, rsp); // NULL
    push(argv0_ptr as _, rsp);

    // push argc
    push(1, rsp);

    log::info!("Leap of Faith!");
    unsafe { processor::jump_to_user_fn(entry as _, current_rsp) }
}

#[cfg(all(test, target_os = "none"))]
#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    test_main();
    testing::exit_qemu(testing::QemuExitCode::Success)
}

#[cfg(all(test, target_os = "none"))]
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    testing::test_panic_handler(info)
}

#[cfg(not(test))]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}");

    let has_terminal = drivers::terminal::TERMINAL.lock().is_some();
    if has_terminal {
        use drivers::terminal::{Color, Reset};
        println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);
    } else {
        dbg_println!("KERNEL PANIC: {info}");
    }

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}
