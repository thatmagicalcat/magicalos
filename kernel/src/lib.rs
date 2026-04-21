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
pub mod volatile;

use alloc::{sync::Arc, vec::Vec};
use kernel::USER_ENTRY;

use elf::*;
use io::Read;

use crate::{
    arch::processor, kernel::{USER_STACK_BOTTOM, USER_STACK_TOP}, memory::paging::PageTableEntryFlags
};

#[rustfmt::skip]
pub(crate) const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

pub fn kentry() {
    kernel::init();

    extern "C" fn helper() {
        load_elf(c"/home/thatmagicalcat/user.elf".as_ptr());
    }

    scheduler::spawn(helper, scheduler::NORMAL_PRIORITY).unwrap();
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

    let elf_arc = Arc::new(elf_data);

    scheduler::with_current_task(|task| {
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
            let offset = ph_off + (i * header.program_header_table_entry_size as usize);
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

            let start_page = utils::align_down(vaddr as usize, memory::PAGE_SIZE);
            let end_page = utils::align_up(vaddr as usize + mem_size, memory::PAGE_SIZE);

            let mut flags = PageTableEntryFlags::USER_ACCESSIBLE;

            if phdr.flags & Elf64ProgramHeaderFlag::Write as u32 != 0 {
                flags |= PageTableEntryFlags::WRITABLE;
            }

            if phdr.flags & Elf64ProgramHeaderFlag::Execute as u32 == 0 {
                flags |= PageTableEntryFlags::NO_EXECUTE;
            }

            task.vmspace.insert(start_page, end_page, flags, memory::MappingType::Elf {
                data: Arc::clone(&elf_arc),
                file_offset,
                file_size,
            }).expect("Failed to insert ELF VMA");
        }
    });

    log::info!("Leap of Faith!");
    unsafe { processor::jump_to_user_fn(entry as _) }
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
