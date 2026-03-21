#![no_std]
#![no_main]
#![feature(allocator_api)]
#![warn(clippy::missing_const_for_fn)]
#![allow(clippy::empty_loop)]

extern crate alloc;

mod apic;
mod gdt;
mod interrupts;
mod ioapic;
#[path = "acpi.rs"]
mod kernel_acpi;
mod macros;
mod memory;
mod port;
mod qemu_debug;
mod serial;
mod task;
mod utils;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    println!("Hello, World!");

    interrupts::init();

    let boot_info = unsafe {
        multiboot2::BootInformation::load(
            multiboot_info_addr as *const multiboot2::BootInformationHeader,
        )
    }
    .expect("Failed to load multiboot info");

    let mut allocator = memory::BitmapFrameAllocator::new(&boot_info);
    memory::paging::remap::kernel(&mut allocator, &boot_info);

    let mut active_table = memory::paging::ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    gdt::init();

    let (rev, rsdt_address) = boot_info
        .rsdp_v2_tag()
        .map(|tag| {
            let xsdt = tag.xsdt_address();
            if xsdt != 0 {
                (2, xsdt)
            } else {
                (0, tag.xsdt_address())
            }
        })
        .or_else(|| boot_info.rsdp_v1_tag().map(|tag| (0, tag.rsdt_address())))
        .expect("Failed to find RSDP tag in multiboot2 info");

    let acpi_tables = unsafe {
        acpi::AcpiTables::from_rsdt(
            kernel_acpi::KernelAcpiHandler::new(alloc::sync::Arc::new(spin::Mutex::new(
                memory::TinyAllocator::<1>::new(&mut allocator),
            ))),
            rev,
            rsdt_address,
        )
        .expect("Failed to parse ACPI tables")
    };

    let Ok((acpi::platform::InterruptModel::Apic(apic_info), _processor_info)) =
        acpi::platform::InterruptModel::new(&acpi_tables)
    else {
        panic!("Unsupported interrupt model");
    };

    apic_info
        .io_apics
        .iter()
        .map(|apic_info: &acpi::platform::interrupt::IoApic| {
            ioapic::IoApic::new(
                apic_info.address as usize,
                apic_info.global_system_interrupt_base as usize,
                active_table.mapper_mut(),
                &mut allocator,
            )
        })
        .for_each(ioapic::register);

    // enable keyboard interrupt
    // TODO: find the correct GSI for the keyboard instead of hardcoding it to 1
    ioapic::enable_irq(1, interrupts::KEYBOARD, apic::get_id());

    apic::init();

    // TODO: do proper calibration of the timer frequency
    apic::init_timer(
        apic::DivideConfig::DIVIDE_BY_16,
        10_000_000,
        apic::LvtTimerMode::PERIODIC,
    );

    let mut executor = task::Executor::new();
    executor.spawn(task::keyboard::print_keypresses());

    executor.run();
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

    writer_lock.change_screen_colors(vga_buffer::Color::White, vga_buffer::Color::Red);
    writer_lock.set_color(vga_buffer::Color::Yellow, vga_buffer::Color::Red);

    drop(writer_lock);

    print!("=== KERNEL PANIC ===\n{}", info);

    loop {}
}
