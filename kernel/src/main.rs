#![no_std]
#![no_main]
#![allow(unused)]
#![warn(clippy::missing_const_for_fn)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]

// SAFETY: trust me bro
unsafe extern "C" {
    safe static kernel_start: [u8; 0];
    safe static kernel_end: [u8; 0];
}

use core::{arch::asm, fmt::Write, ptr};

use multiboot2 as mb2;

use vga_buffer::{Buffer, Color, Writer};

use crate::memory::{
    FrameAllocator,
    paging::{self, EntryFlags, VirtualAddress},
};

mod interrupts;
mod macros;
mod memory;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    interrupts::init();
    println!("Hello, World!");

    unsafe {
        asm!("syscall");
    }

    let boot_info = unsafe {
        mb2::BootInformation::load(multiboot_info_addr as *const mb2::BootInformationHeader)
    }
    .expect("Failed to load multiboot info");

    // print_memory_areas(&boot_info);

    let mut frame_allocator = memory::BitmapFrameAllocator::new(&boot_info);
    let mut active_page_tbl = paging::ActivePageTable::new();
    let addr = VirtualAddress(42 * 512 * 512 * 4096); // 42th P3 entry
    let frame = frame_allocator
        .allocate_frame()
        .expect("Failed to allocate frame");
    println!(
        "[1] Virtual: {:p}, mapped to {:?}",
        *addr as *const (), frame
    );

    active_page_tbl.map_to(addr, frame, EntryFlags::empty(), &mut frame_allocator);
    println!(
        "[2] Physical address: {:?}",
        active_page_tbl.translate(addr).map(|p| *p as *const ())
    );
    println!("[3] {:?} address: {:?}", frame, frame.get_ptr());
    println!("[4] Read before unmap: {}", unsafe {
        ptr::read_volatile(*addr as *const u64)
    });
    println!(
        "[5] Next free frame: {:?}",
        frame_allocator.allocate_frame()
    );

    active_page_tbl.unmap(addr, &mut frame_allocator);

    // try to read the unmapped address, should cause a page fault
    println!("[5] Read after unmap: {}", unsafe {
        ptr::read_volatile(*addr as *const u64)
    });

    #[allow(clippy::empty_loop)]
    loop {}
}

fn print_memory_areas(boot_info: &multiboot2::BootInformation<'_>) {
    let memory_map_tag = boot_info
        .memory_map_tag()
        .expect("Memory map tag not found in multiboot info");

    println!("Memory areas:");
    for area in memory_map_tag.memory_areas() {
        println!(
            "  - start: {:#010x}, end: {:#010x}, size: {} KB, type: {:?}",
            area.start_address(),
            area.end_address(),
            (area.end_address() - area.start_address()) / 1024,
            area.typ()
        );
    }
}

/// Returns the start and end addresses of the kernel in memory.
pub fn kernel_bounds() -> (usize, usize) {
    unsafe {
        (
            &kernel_start as *const _ as usize,
            &kernel_end as *const _ as usize,
        )
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    vga_buffer::WRITER
        .lock()
        .set_color(Color::LightRed, Color::Black);
    println!("Panic: {}", info);

    loop {}
}
