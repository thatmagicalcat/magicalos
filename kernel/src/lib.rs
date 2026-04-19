#![no_std]
#![cfg_attr(all(test, target_os = "none"), no_main)]
#![warn(clippy::missing_const_for_fn)]
#![feature(custom_test_frameworks)]
#![feature(linked_list_cursors)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

#[cfg(not(test))]
use alloc::vec::Vec;
#[cfg(not(test))]
use arch::processor;
#[cfg(not(test))]
use scheduler::NORMAL_PRIORITY;

#[cfg(not(test))]
use elf::*;
#[cfg(not(test))]
use io::Read;

#[cfg(not(test))]
use crate::memory::paging::{PageTable, PageTableEntryFlags, VirtualAddress};

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

#[rustfmt::skip]
pub(crate) const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

#[cfg(not(test))]
pub fn kernel_entry() -> ! {
    kernel::init();

    extern "C" fn helper() {
        load_elf(c"/home/thatmagicalcat/user.elf".as_ptr());
    }

    scheduler::spawn(helper, NORMAL_PRIORITY).unwrap();
    scheduler::reschedule();

    log::error!("Scheduler empty, main kernel thread entering idle loop");

    halt_loop()
}

pub fn halt_loop() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[cfg(not(test))]
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

    let header: Elf64Header =
        unsafe { core::ptr::read_unaligned(elf_data.as_ptr() as *const Elf64Header) };

    assert!(header.ident.magic == ELF_OBJECT_MAGIC);

    let entry = header.entry;
    let ph_off = header.program_header_table_offset as usize;
    let ph_num = header.program_header_table_num_entires as usize;
    let hhdm_offset = unsafe { (*limine_requests::HHDM_REQUEST.response).offset } as usize;

    let mut highest_addr_page = 0;

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

        highest_addr_page = highest_addr_page.max(end_page);

        let mut remaining_file_size = file_size;
        let mut current_file_offset = file_offset;

        for i in 0..pages {
            let current_vpage_addr = (start_page + (i * memory::PAGE_SIZE)) as u64;
            let frame = memory::allocate_frame().expect("OOM Loading ELF segment");

            PageTable::active().mapper_mut().map_to(
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

    log::info!("Leap of Faith!");
    unsafe { processor::jump_to_user_fn(entry as _) }
}

pub fn panic(info: &PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info}");

    let has_terminal = drivers::terminal::TERMINAL.lock().is_some();
    if has_terminal {
        use drivers::terminal::{Color, Reset};
        println!("{}KERNEL PANIC: {}{}", Color::Red.bg(), info, Reset);
    } else {
        dbg_println!("KERNEL PANIC: {info}");
    }

    halt_loop()
}

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        dbg_print!("test {} ... ", core::any::type_name::<T>());
        self();
        dbg_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    testing::ensure_test_kernel_init();
    dbg_println!("running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    dbg_println!("all tests passed");
    testing::exit_qemu(testing::QemuExitCode::Success)
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    dbg_println!("[failed]");
    dbg_println!("{}", info);
    testing::exit_qemu(testing::QemuExitCode::Failed)
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
    test_panic_handler(info)
}
