#![no_std]
#![no_main]
#![feature(allocator_api)]
#![warn(clippy::missing_const_for_fn)]
#![allow(clippy::empty_loop)]

extern crate alloc;

use alloc::{collections::BTreeMap, vec};
use memory::paging::ActivePageTable;
use multiboot2 as mb2;
use vga_buffer::Color;

mod interrupts;
mod gdt;
mod macros;
mod memory;
mod utils;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    interrupts::init();
    gdt::init();

    println!("Hello, World!");

    // unsafe {
    //     core::arch::asm! {
    //         "call 2f",
    //         "2:",
    //         "call 2b",
    //
    //         options(nomem, noreturn)
    //     }
    // };

    let boot_info = unsafe {
        mb2::BootInformation::load(multiboot_info_addr as *const mb2::BootInformationHeader)
    }
    .expect("Failed to load multiboot info");

    let mut allocator = memory::BitmapFrameAllocator::new(&boot_info);
    memory::paging::remap::kernel(&mut allocator, &boot_info);
    let mut active_table = ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    let mut v = vec![1, 2, 3];
    v.push(69);

    println!("{v:?}");

    let mut map = BTreeMap::new();
    map.insert("A", 1);
    map.insert("B", 2);

    println!("{map:?}");

    loop {}
}

unsafe extern "C" {
    static kernel_start: [u8; 0];
    static kernel_end: [u8; 0];
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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut writer_lock = vga_buffer::WRITER.lock();

    writer_lock.change_screen_colors(Color::White, Color::Red);
    writer_lock.set_color(Color::Yellow, Color::Red);

    drop(writer_lock);

    print!("=== KERNEL PANIC ===\n{}", info);

    loop {}
}
