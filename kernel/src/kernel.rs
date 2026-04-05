//! Nothing interesting here, just the main kernel initialization code and panic handler. The real
//! fun starts in the modules :)
//! This file exists because I want to keep main.rs clean

use core::alloc::Layout;
use core::{ffi, ptr};

use alloc::alloc::{alloc, dealloc};

use crate::limine_requests::*;
use crate::*;

pub fn init() {
    init_logging();
    gdt::init();
    interrupts::init();

    log::info!("{:p}", MEMMAP.response);
    log::info!("{}", unsafe { (*MEMMAP.response).entry_count });

    let memmap = unsafe {
        let response = &*MEMMAP.response;
        core::slice::from_raw_parts(response.entries, response.entry_count as usize)
    };

    log_memmap(memmap);
    let mut allocator = memory::BitmapFrameAllocator::new(memmap);

    let mut active_table = memory::paging::ActivePageTable::new();
    memory::heap::init(active_table.mapper_mut(), &mut allocator);

    let acpi_tables = parse_acpi_tables();
    register_ioapics(&acpi_tables, &mut allocator, &mut active_table);

    let hpet_info =
        acpi::HpetInfo::new(&acpi_tables).expect("Failed to find HPET info in ACPI tables");
    log::info!("HPET info: {:?}", hpet_info);

    let base_addr = hpet_info.base_address;
    let hpet = hpet::HPET
        .call_once(|| hpet::Hpet::new(base_addr, active_table.mapper_mut(), &mut allocator));

    log::info!("HPET frequency: {} MHz", 1_000_000_000 / hpet.time_period);

    io::apic::init(active_table.mapper_mut(), &mut allocator);
    io::apic::calibrate_lapic_timer(hpet);

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

    let flanterm_ctx = flanterm_console_init();
    log::info!("erm");

    unsafe {
        let t = "Hello, World!\n\r";
        flanterm::flanterm_write(flanterm_ctx, t.as_ptr() as _, t.len());
    }

    let timer_cfg = utils::duration_to_timer_config(
        core::time::Duration::from_millis(10).as_nanos() as _,
        io::apic::get_timer_frequency(),
    )
    .expect("Duration too long to convert to ticks");

    scheduler::init();

    // slap kernel after every 10ms :)
    io::apic::set_timer(
        timer_cfg.divide_config,
        timer_cfg.initial_count,
        io::apic::LvtTimerMode::PERIODIC,
    );

    // start slapping lol!
    interrupts::enable_interrupts();
}

fn flanterm_console_init() -> *mut flanterm::flanterm_context {
    let fb = unsafe {
        assert!(!FRAMEBUFFER_REQUEST.response.is_null());

        let response = &*FRAMEBUFFER_REQUEST.response;

        assert!(response.framebuffer_count > 0, "No framebuffer found");
        assert!(
            !response.framebuffers.is_null(),
            "Framebuffers array pointer is null"
        );

        let first_fb = *response.framebuffers;
        assert!(!first_fb.is_null(), "First framebuffer pointer is null");

        &*first_fb
    };

    // let mut params = None::<limine::limine_flanterm_fb_init_params>;
    let mut params = unsafe {
        let resp = FLANTERM_FB_INIT_PARAMS_REQUEST.response;
        if resp.is_null() {
            log::error!("Flanterm init parameters are not provided by the bootloader");
            None
        } else if (*resp).entry_count == 0 || (*resp).entries.is_null() {
            log::error!("Flanterm init parameters entry list is empty or null");
            None
        } else {
            Some(**(*resp).entries)
        }
    };

    let (
        canvas,
        ansi_colours,
        ansi_bright_colours,
        default_bg,
        default_fg,
        default_bg_bright,
        default_fg_bright,
        font,
        font_width,
        font_height,
        font_spacing,
        font_scale_x,
        font_scale_y,
        margin,
        rotation,
    ) = if let Some(ref mut p) = params {
        (
            p.canvas,
            p.ansi_colours.as_mut_ptr(),
            p.ansi_bright_colours.as_mut_ptr(),
            &raw mut p.default_bg,
            &raw mut p.default_fg,
            &raw mut p.default_bg_bright,
            &raw mut p.default_fg_bright,
            p.font,
            p.font_width as _,
            p.font_height as _,
            p.font_spacing as _,
            p.font_scale_x as _,
            p.font_scale_y as _,
            p.margin as _,
            p.rotation as _,
        )
    } else {
        (
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
            1,
            0,
            0,
            0,
            0,
        )
    };

    unsafe {
        extern "C" fn kmalloc(size: usize) -> *mut ffi::c_void {
            unsafe { alloc(Layout::from_size_align_unchecked(size, 1)) as _ }
        }

        extern "C" fn kfree(ptr: *mut ffi::c_void, size: usize) {
            unsafe { dealloc(ptr as _, Layout::from_size_align_unchecked(size, 1)) };
        }

        flanterm::flanterm_fb_init(
            Some(kmalloc),
            Some(kfree),
            fb.address as _,
            fb.width as _,
            fb.height as _,
            fb.pitch as _,
            fb.red_mask_size as _,
            fb.red_mask_shift,
            fb.green_mask_size,
            fb.green_mask_shift,
            fb.blue_mask_size,
            fb.blue_mask_shift,
            canvas,
            ansi_colours,
            ansi_bright_colours,
            default_bg,
            default_fg,
            default_bg_bright,
            default_fg_bright,
            font,
            font_width,
            font_height,
            font_spacing,
            font_scale_x,
            font_scale_y,
            margin,
            rotation,
        )
    }
}

fn log_memmap(memory_map: &[*mut limine::limine_memmap_entry]) {
    let hhdm_offset = unsafe { (*HHDM_REQUEST.response).offset };
    log::info!("Memory areas:");

    for entry in memory_map.iter().map(|e| unsafe { &**e }) {
        let virtual_start = entry.base + hhdm_offset;

        log::info!(
            "  - virt {virtual_start:#010x} -> phys {:#010x}, size: {} KiB, type: {}",
            entry.base,
            entry.length / 1024,
            match entry.type_ as u32 {
                limine::LIMINE_MEMMAP_ACPI_NVS => "ACPI NVS",
                limine::LIMINE_MEMMAP_ACPI_RECLAIMABLE => "ACPI RECLAIMABLE",
                limine::LIMINE_MEMMAP_BAD_MEMORY => "BAD MEMORY",
                limine::LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE => "BOOTLOADER RECLAIMABLE",
                limine::LIMINE_MEMMAP_EXECUTABLE_AND_MODULES => "KERNEL",
                limine::LIMINE_MEMMAP_FRAMEBUFFER => "FRAMEBUFFER",
                limine::LIMINE_MEMMAP_RESERVED => "MAPPED RESERVED",
                limine::LIMINE_MEMMAP_RESERVED_MAPPED => "RESERVED MAPPED",
                limine::LIMINE_MEMMAP_USABLE => "USABLE",

                _ => unreachable!(),
            }
        );
    }
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
    acpi_tables: &acpi::AcpiTables<io::acpi::KernelAcpiHandler>,
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

fn parse_acpi_tables() -> acpi::AcpiTables<io::acpi::KernelAcpiHandler> {
    log::info!("Parsing ACPI tables");

    let response = unsafe { &*RSDP_REQUEST.response };
    let rsdp_phys = response.address as usize - unsafe { (*HHDM_REQUEST.response).offset as usize };

    log::info!("RSDP physical address: {:#010x}", rsdp_phys);

    unsafe {
        acpi::AcpiTables::from_rsdp(io::acpi::KernelAcpiHandler, rsdp_phys as _)
            .expect("Failed to parse ACPI tables")
    }
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
