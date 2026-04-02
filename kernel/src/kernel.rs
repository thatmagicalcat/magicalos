//! Nothing interesting here, just the main kernel initialization code and panic handler. The real
//! fun starts in the modules :)
//! This file exists because I want to keep main.rs clean

use crate::*;

pub fn init(multiboot_info_addr: u32) {
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
    memory::paging::remap::remap(&mut allocator, &boot_info);

    let mut active_table = memory::paging::ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    let Some(Ok(fb_tag)) = boot_info.framebuffer_tag() else {
        panic!("Framebuffer tag not found in multiboot info / unsupported framebuffer format");
    };

    log::info!(
        "Framebuffer: {}x{}, depth: {} bpp, pitch: {} bytes, address: {:#010x}",
        fb_tag.width(),
        fb_tag.height(),
        fb_tag.bpp(),
        fb_tag.pitch(),
        fb_tag.address(),
    );

    let format = fb_tag
        .buffer_type()
        .expect("Unsupported framebuffer format");

    match format {
        multiboot2::FramebufferType::RGB { red, green, blue } => {
            log::info!(
                "Framebuffer format: RGB, (position, size) red: ({}, {}), green: ({}, {}), blue: ({}, {})",
                red.position,
                red.size,
                green.position,
                green.size,
                blue.position,
                blue.size
            );

            log::info!("Initializing graphics console with PSF2 font");
            graphics::init_window_console(
                graphics::FrameBufferInfo {
                    width: fb_tag.width(),
                    height: fb_tag.height(),
                    bits_per_pixel: fb_tag.bpp(),
                    pitch: fb_tag.pitch(),
                    r_shift: red.position,
                    g_shift: green.position,
                    b_shift: blue.position,
                },
                graphics::PSF2Font::new(FONT_DATA).expect("Failed to load PSF2 font"),
            );
        }

        _ => panic!("Unsupported framebuffer format"),
    };

    gdt::init();

    let acpi_tables = parse_acpi_tables(boot_info, &mut allocator);
    register_ioapics(&acpi_tables, &mut allocator, &mut active_table);

    let hpet_info =
        acpi::HpetInfo::new(&acpi_tables).expect("Failed to find HPET info in ACPI tables");
    log::info!("HPET info: {:?}", hpet_info);

    let base_addr = hpet_info.base_address;
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

    let pci_devices = io::pci::enumerate();
    log::info!("Found {} PCI devices:", pci_devices.len());
    for device in pci_devices {
        log::info!("  - {}", device);
    }

    io::apic::init();
    io::apic::calibrate_lapic_timer(hpet);

    // let timer_cfg = utils::duration_to_timer_config(
    //     core::time::Duration::from_millis(10).as_nanos() as _,
    //     io::apic::get_timer_frequency(),
    // )
    // .expect("Duration too long to convert to ticks");

    // for x in 0..screen.width {
    //     for y in 0..screen.height {
    //         let r = (x * 255 / screen.width) as u8;
    //         let g = (y * 255 / screen.height) as u8;
    //         let b = 128;
    //
    //         screen.write_pixel(x, y, r, g, b);
    //     }
    // }

    // scheduler::init();
    //
    // scheduler::spawn(Thread::new(|| f(Duration::from_millis(100), "Thread 1 ->")));
    // scheduler::spawn(Thread::new(|| f(Duration::from_millis(100), "Thread 2 ->")));
    //
    // scheduler::spawn(Thread::new(|| {
    //     let mut executor = task::Executor::new();
    //
    //     executor.spawn(task::keyboard::print_keypresses());
    //
    //     executor.run();
    // }));
    //
    // // slap kernel after every 10ms :)
    // io::apic::set_timer(
    //     timer_cfg.divide_config,
    //     timer_cfg.initial_count,
    //     io::apic::LvtTimerMode::PERIODIC,
    // );

    // start slapping lol!
    interrupts::enable_interrupts();
}

// fn f(d: Duration, msg: &str) {
//     let mut count = 0;
//
//     loop {
//         scheduler::sleep(d);
//         count += 1;
//         println!("{msg} {count}");
//     }
// }

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
    let mut writer_lock = interrupts::without_interrupts(|| vga_buffer::WRITER.lock());

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

                let file = record
                    .file()
                    .unwrap_or("?")
                    .rsplit('/')
                    .next()
                    .unwrap_or("?");
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
