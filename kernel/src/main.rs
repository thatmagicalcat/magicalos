#![no_std]
#![no_main]
#![feature(allocator_api)]
#![warn(clippy::missing_const_for_fn)]
#![allow(clippy::empty_loop)]

extern crate alloc;

use memory::paging::ActivePageTable;
use multiboot2 as mb2;
use vga_buffer::Color;

use crate::utils::enable_interrupts;

mod apic;
mod gdt;
mod interrupts;
mod macros;
mod memory;
mod port;
mod utils;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    println!("Hello, World!");

    interrupts::init();
    gdt::init();

    let boot_info = unsafe {
        mb2::BootInformation::load(multiboot_info_addr as *const mb2::BootInformationHeader)
    }
    .expect("Failed to load multiboot info");

    let mut allocator = memory::BitmapFrameAllocator::new(&boot_info);
    memory::paging::remap::kernel(&mut allocator, &boot_info);

    apic::init();
    apic::init_timer(
        apic::DivideConfig::DIVIDE_BY_16,
        10_000_000,
        apic::LvtTimerMode::PERIODIC,
    );

    let mut active_table = ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    enable_interrupts();
    loop {}
}

unsafe extern "C" {
    static kernel_start: [u8; 0];
    static kernel_end: [u8; 0];
}

pub struct KernelBounds {
    pub start: usize,
    pub end: usize,
}

/// Returns the start and end addresses of the kernel in memory.
pub fn kernel_bounds() -> KernelBounds {
    unsafe {
        KernelBounds {
            start: kernel_start.as_ptr() as usize,
            end: kernel_end.as_ptr() as usize,
        }
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
