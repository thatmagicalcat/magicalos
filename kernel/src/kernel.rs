use crate::arch::*;
use crate::limine_requests::*;
use crate::memory::paging::{PageTable, VirtualAddress};
use crate::*;

/// The entry point of user tasks
pub const USER_ENTRY: VirtualAddress = VirtualAddress(0x400000_u64);
pub const USER_STACK_TOP: VirtualAddress = VirtualAddress(0x0000_7FFF_FFFF_F000_u64);
pub const USER_STACK_BOTTOM: VirtualAddress =
    VirtualAddress(USER_STACK_TOP.0 - MAX_USER_STACK_SIZE);
pub const MAX_USER_STACK_SIZE: u64 = 8 * 1024 * 1024; // 8 MiB

/// The kernel's page table. This is used for mapping the kernel's virtual address space to
/// physical memory.
pub static mut KERNEL_PAGE_TABLE: Option<PageTable> = None;

/// SAFETY: This function should only be called after the kernel page table has been initialized in
/// `init()`
#[allow(static_mut_refs)]
pub fn get_kernel_page_table() -> &'static mut PageTable {
    unsafe {
        KERNEL_PAGE_TABLE
            .as_mut()
            .expect("Kernel page table not initialized")
    }
}

pub fn init() {
    init_logging();
    log::info!("Hello, World!");

    processor::init();

    unsafe {
        // the currently active page table is the kernel's page table
        KERNEL_PAGE_TABLE = Some(PageTable::active());
    }

    gdt::init();
    interrupts::init();

    let memmap = unsafe {
        let response = &*MEMMAP.response;
        core::slice::from_raw_parts(response.entries, response.entry_count as usize)
    };

    log_memmap(memmap);
    memory::init_global_frame_allocator(memmap);

    // locking here because modules like heap are gonna allocate a ton of pages
    // so locking and unlocking on every `allocate_frame` might be a tiny bit slow :)
    let mut allocator = memory::lock_global_frame_allocator();

    let kernel_page_table = get_kernel_page_table();
    //////////////////////////////////// HEAP INIT
    memory::heap::init(kernel_page_table.mapper_mut(), &mut *allocator);

    memory::init_vmm();
    syscall::init();
    drivers::terminal::init();
    fs::init_vfs();

    let acpi_tables = parse_acpi_tables();
    ioapic::register_ioapics(&acpi_tables, &mut *allocator, kernel_page_table);

    let hpet_info =
        acpi::HpetInfo::new(&acpi_tables).expect("Failed to find HPET info in ACPI tables");
    log::info!("HPET info: {:#?}", hpet_info);

    let base_addr = hpet_info.base_address;
    let hpet = hpet::HPET
        .call_once(|| hpet::Hpet::new(base_addr, kernel_page_table.mapper_mut(), &mut *allocator));

    log::info!("HPET frequency: {} MHz", 1_000_000_000 / hpet.time_period);

    apic::init(kernel_page_table.mapper_mut(), &mut *allocator);
    apic::calibrate_lapic_timer(hpet);

    // IMPORTANT!
    drop(allocator);

    drivers::keyboard::init();

    let pci_devices = bus::pci::enumerate();
    log::info!("Found {} PCI devices:", pci_devices.len());

    for device in pci_devices {
        log::info!("  - {}", device);
    }

    scheduler::init();

    let timer_cfg = utils::duration_to_timer_config(
        core::time::Duration::from_millis(10).as_nanos() as _,
        apic::get_timer_frequency(),
    )
    .expect("Duration too long to convert to ticks");

    // slap kernel after every 10ms :)
    apic::set_timer(
        timer_cfg.divide_config,
        timer_cfg.initial_count,
        apic::LvtTimerMode::PERIODIC,
    );

    // start slapping lol!
    interrupts::enable_interrupts();
}

pub fn init_for_tests() {
    init_logging();
    log::info!("Initializing test kernel environment");

    processor::init();

    unsafe {
        KERNEL_PAGE_TABLE = Some(PageTable::active());
    }

    gdt::init();
    interrupts::init();

    let memmap = unsafe {
        let response = &*MEMMAP.response;
        core::slice::from_raw_parts(response.entries, response.entry_count as usize)
    };

    log_memmap(memmap);
    memory::init_global_frame_allocator(memmap);

    let mut allocator = memory::lock_global_frame_allocator();
    let kernel_page_table = get_kernel_page_table();

    memory::heap::init(kernel_page_table.mapper_mut(), &mut *allocator);
    memory::init_vmm();
    syscall::init();
    scheduler::init();
}

fn log_memmap(memory_map: &[*mut limine::limine_memmap_entry]) {
    let hhdm_offset = unsafe { (*HHDM_REQUEST.response).offset };
    log::info!("Memory areas:     virt -> phys       |           size | type");
    for entry in memory_map.iter().map(|e| unsafe { &**e }) {
        let virtual_start = entry.base + hhdm_offset;
        log::info!(
            "  * {virtual_start:#010x} -> {:#010x} | {:>10} KiB | {}",
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

fn parse_acpi_tables() -> acpi::AcpiTables<bus::acpi::KernelAcpiHandler> {
    log::info!("Parsing ACPI tables");

    let response = unsafe { &*RSDP_REQUEST.response };
    let rsdp_phys = response.address as usize - unsafe { (*HHDM_REQUEST.response).offset as usize };

    log::info!("RSDP physical address: {:#010x}", rsdp_phys);

    unsafe {
        acpi::AcpiTables::from_rsdp(bus::acpi::KernelAcpiHandler, rsdp_phys as _)
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

                let file = &record.file().unwrap_or("?")["kernel/src/".len()..];
                let line = record.line().unwrap_or(0);

                dbg_println!(
                    "{level_color}[{: <5}]{reset} @ {meta_color}\x1b[3m<{file}:{line}>\x1b[23m{reset} {level_color}{}{reset}",
                    // "{level_color}[{: <5}]{reset} {reset}{level_color}{}{reset}",
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
