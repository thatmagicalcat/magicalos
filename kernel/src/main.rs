#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(abi_x86_interrupt)]
#![warn(clippy::missing_const_for_fn)]
#![allow(clippy::empty_loop)]

use core::time::Duration;

use crate::task::timer::{self, sleep};

#[rustfmt::skip]
const MIN_LOG_LEVEL: log::LevelFilter = {
    #[cfg(log_level = "trace")] { log::LevelFilter::Trace }
    #[cfg(log_level = "debug")] { log::LevelFilter::Debug }
    #[cfg(log_level = "info")] { log::LevelFilter::Info }
    #[cfg(log_level = "warn")] { log::LevelFilter::Warn }
    #[cfg(log_level = "error")] { log::LevelFilter::Error }
};

extern crate alloc;

mod gdt;
mod hpet;
mod interrupts;
mod io;
mod ioapic;
mod macros;
mod memory;
mod task;
mod utils;
mod vga_buffer;
mod volatile;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(multiboot_info_addr: u32) -> ! {
    init_logging();

    interrupts::init();

    let boot_info = unsafe {
        multiboot2::BootInformation::load(
            multiboot_info_addr as *const multiboot2::BootInformationHeader,
        )
    }
    .expect("Failed to load multiboot info");

    log_memory_areas(&boot_info);

    let mut allocator = memory::BitmapFrameAllocator::new(&boot_info);
    memory::paging::remap::kernel(&mut allocator, &boot_info);

    let mut active_table = memory::paging::ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    gdt::init();

    let acpi_tables = parse_acpi_tables(boot_info, &mut allocator);
    register_ioapics(&acpi_tables, &mut allocator, &mut active_table);

    let hpet_info =
        acpi::HpetInfo::new(&acpi_tables).expect("Failed to find HPET info in ACPI tables");
    log::info!("HPET info: {:?}", hpet_info);
    let base_addr = hpet_info.base_address as usize;
    let hpet = hpet::HPET
        .call_once(|| hpet::Hpet::new(base_addr, active_table.mapper_mut(), &mut allocator));

    log::info!("HPET frequency: {} MHz", 1_000_000_000 / hpet.time_period);

    // enable keyboard interrupt
    // TODO: find the correct GSI for the keyboard instead of hardcoding it to 1
    ioapic::enable_irq(
        1,
        interrupts::InterruptEntryType::Keyboard as _,
        io::apic::get_id(),
    );

    io::apic::init();
    io::apic::calibrate_lapic_timer(hpet);

    let mut executor = task::Executor::new();
    // responsible for handling sleep timers
    executor.spawn(timer::timer_dispatch());

    async fn f(secs: u64) {
        sleep(Duration::from_secs(secs)).await;
        println!("Printing after {secs} seconds");
    }

    executor.spawn(f(1));
    executor.spawn(f(2));
    executor.spawn(f(3));
    executor.spawn(f(4));
    executor.spawn(f(5));
    executor.spawn(f(6));
    executor.spawn(f(7));

    executor.run();
}

fn register_ioapics(
    acpi_tables: &acpi::AcpiTables<io::acpi::KernelAcpiHandler<1>>,
    allocator: &mut memory::BitmapFrameAllocator,
    active_table: &mut memory::paging::ActivePageTable,
) {
    let Ok((acpi::platform::InterruptModel::Apic(apic_info), _processor_info)) =
        acpi::platform::InterruptModel::new(acpi_tables)
    else {
        panic!("Unsupported interrupt model");
    };

    log::info!("Registering IO APICs...");

    apic_info
        .io_apics
        .iter()
        .map(|apic_info: &acpi::platform::interrupt::IoApic| {
            ioapic::IoApic::new(
                apic_info.address as usize,
                apic_info.global_system_interrupt_base as usize,
                active_table.mapper_mut(),
                apic_info.id,
                allocator,
            )
        })
        .for_each(ioapic::register);
}

fn parse_acpi_tables(
    boot_info: multiboot2::BootInformation<'_>,
    allocator: &mut memory::BitmapFrameAllocator,
) -> acpi::AcpiTables<io::acpi::KernelAcpiHandler<1>> {
    log::info!("Parsing ACPI tables");
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

    log::info!(
        "ACPI revision: {}, RSDT/XSDT address: {:#010x}",
        rev,
        rsdt_address
    );

    unsafe {
        acpi::AcpiTables::from_rsdt(
            io::acpi::KernelAcpiHandler::new(alloc::sync::Arc::new(spin::Mutex::new(
                memory::TinyAllocator::<1>::new(allocator),
            ))),
            rev,
            rsdt_address,
        )
        .expect("Failed to parse ACPI tables")
    }
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

fn log_memory_areas(boot_info: &multiboot2::BootInformation<'_>) {
    let memory_map_tag = boot_info
        .memory_map_tag()
        .expect("Memory map tag not found in multiboot info");

    log::info!("Memory areas:");
    for area in memory_map_tag.memory_areas() {
        log::info!(
            "  - start: {:#010x}, end: {:#010x}, size: {} KiB, type: {:?}",
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
    log::error!("KERNEL PANIC: {}", info);

    loop {}
}

pub fn init_logging() {
    struct KernelLogger;
    impl log::Log for KernelLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= MIN_LOG_LEVEL
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                let level_color = match record.level() {
                    log::Level::Error => "\x1b[91m", // Bright Red
                    log::Level::Warn => "\x1b[93m",  // Bright Yellow
                    log::Level::Info => "\x1b[92m",  // Bright Green
                    log::Level::Debug => "\x1b[96m", // Bright Cyan
                    log::Level::Trace => "\x1b[95m", // Bright Magenta
                };

                let meta_color = "\x1b[90m"; // Dim Gray for file/line
                let reset = "\x1b[0m";

                let file = record.file().unwrap_or("?");
                let line = record.line().unwrap_or(0);

                dbg_println!(
                    "{level_color}[{: <5}]{reset} {meta_color}[{file}:{line}] {reset}{level_color}{}{reset}",
                    record.level(),
                    record.args(),
                );
            }
        }

        fn flush(&self) {}
    }

    static LOGGER: KernelLogger = KernelLogger;
    log::set_logger(&LOGGER).expect("Failed to set logger");
    log::set_max_level(log::LevelFilter::Trace);
}
