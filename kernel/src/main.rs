#![no_std]
#![no_main]
#![warn(clippy::missing_const_for_fn)]
#![feature(linked_list_cursors)]

use alloc::vec::Vec;
use arch::processor;
use drivers::terminal::{Color, Reset};
use scheduler::NORMAL_PRIORITY;
use syscall::Syscall;

use elf::*;
use io::Read;

use crate::memory::paging::{PageTable, PageTableEntryFlags, VirtualAddress};

extern crate alloc;

mod arch;
mod async_rt;
mod bus;
mod drivers;
mod elf;
mod errno;
mod fd;
mod fs;
mod io;
mod kernel;
mod limine_requests;
mod macros;
mod memory;
mod scheduler;
mod synch;
mod syscall;
mod utils;
mod volatile;

#[rustfmt::skip]
const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    kernel::init();

    scheduler::spawn(create_user_process, NORMAL_PRIORITY).unwrap();
    scheduler::reschedule();

    log::error!("Scheduler empty, main kernel thread entering idle loop");

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "C" fn create_user_process() {
    utils::write_cr3(*memory::paging::user::create_page_table() as _);
    // memory::paging::user::map_user_entry(user_process);

    let path = "/home/thatmagicalcat/user.elf";
    let mut elf_data = Vec::new();
    let mut file = fs::File::open(path).expect("Failed to open user ELF file");
    file.read_to_end(&mut elf_data)
        .expect("Failed to read user ELF file");

    if elf_data.len() < core::mem::size_of::<Elf64Header>() {
        panic!("Buffer too small");
    }

    // Instead of casting to a reference, we READ the data into a local variable.
    // This handles both the 'packed' layout and any alignment issues.
    let header: Elf64Header =
        unsafe { core::ptr::read_unaligned(elf_data.as_ptr() as *const Elf64Header) };

    assert!(header.ident.magic == ELF_OBJECT_MAGIC);

    // Now 'header' is a normal, stack-allocated struct you can use safely.
    let entry = header.entry;
    let ph_off = header.program_header_table_offset as usize;
    let ph_num = header.program_header_table_num_entires as usize;
    let hhdm_offset = unsafe { (*limine_requests::HHDM_REQUEST.response).offset } as usize;

    // To get the Program Headers:
    for i in 0..ph_num {
        let offset = ph_off + (i * header.program_header_table_entry_size as usize);
        let phdr: Elf64ProgramHeader = unsafe {
            core::ptr::read_unaligned(elf_data.as_ptr().add(offset) as *const Elf64ProgramHeader)
        };

        if phdr.type_ != Elf64ProgramHeaderType::Load {
            log::warn!("PHDR type: {:?} is not supported yet", phdr.type_);
            continue;
        }

        let vaddr = phdr.virual_address as usize;
        let mem_size = phdr.mem_size as usize;
        let file_size = phdr.file_size as usize;
        let file_offset = phdr.offset as usize;
        let start_page = utils::align_down(vaddr as usize, memory::PAGE_SIZE);
        let end_page = utils::align_up(vaddr as usize + mem_size, memory::PAGE_SIZE);
        let pages = (end_page - start_page) / memory::PAGE_SIZE;

        let mut remaining_file_size = file_size;
        let mut current_file_offset = file_offset;

        for i in 0..pages {
            let current_vpage_addr = (start_page + (i * memory::PAGE_SIZE)) as u64;
            let frame = memory::allocate_frame().expect("OOM Loading ELF segment");

            PageTable::active()
                .mapper_mut()
                .map_to(
                    VirtualAddress(current_vpage_addr),
                    frame,
                    PageTableEntryFlags::USER_ACCESSIBLE | PageTableEntryFlags::WRITABLE,
                    &mut *memory::lock_global_frame_allocator(),
                );

            let dest_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    (frame.start_address() + hhdm_offset) as *mut u8,
                    memory::PAGE_SIZE,
                )
            };

            dest_slice.fill(0);

            let page_offset = if i == 0 {
                vaddr as usize % memory::PAGE_SIZE
            } else {
                0
            };

            let bytes_to_copy = remaining_file_size.min(memory::PAGE_SIZE - page_offset);

            dest_slice[page_offset..page_offset + bytes_to_copy].copy_from_slice(
                &elf_data[current_file_offset..(current_file_offset + bytes_to_copy)],
            );

            remaining_file_size -= bytes_to_copy;
            current_file_offset += bytes_to_copy;
        }
    }

    log::info!("JUMP!");
    unsafe { processor::jump_to_user_fn(entry as _) }
}

//
// extern "C" fn user_process() {
//     // NOTE: println uses kernel's terminal driver which is located in RING 0
//     // memory, using it will cause a protection violation.
//
//     let msg = *b"Hello from a userspace process!\r\n";
//
//     syscall!(Syscall::Write, fd::STDOUT_FILENO, msg.as_ptr(), msg.len());
//     syscall!(Syscall::Exit);
// }

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}",);
    println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);
    loop {}
}
