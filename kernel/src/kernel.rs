use crate::limine_requests::*;
use crate::memory::FrameAllocator;
use crate::memory::paging::{Mapper, PageTable, VirtualAddress};
use crate::*;
use crate::arch::*;

/// The entry point of user tasks
pub const USER_ENTRY: VirtualAddress = VirtualAddress(0x20000000000_u64);

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
    memory::heap::init(kernel_page_table.mapper_mut(), &mut *allocator);
    memory::init_vmm();

    syscall::init();

    terminal::init();

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

    // enable keyboard interrupt
    // TODO: find the correct GSI for the keyboard instead of hardcoding it to 1
    ioapic::enable_irq(
        1,
        interrupts::InterruptEntryType::Keyboard as _,
        apic::get_id(),
    );

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

// fn f(d: Duration, msg: &str) {
//     let mut count = 0;
//
//     loop {
//         scheduler::sleep(d);
//         count += 1;
//         println!("{msg} {count}");
//     }
// }

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

                let file = record
                    .file()
                    .unwrap_or("?")
                    .rsplit('/')
                    .next()
                    .unwrap_or("?");
                let line = record.line().unwrap_or(0);

                dbg_println!(
                    "{level_color}[{: <5}]{reset} @ <{meta_color}\x1b[3m{file}:{line}\x1b[23m> {reset}{level_color}{}{reset}",
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

// pub fn init_user_land<A: FrameAllocator>(allocator: &mut A) {
//     log::info!("Initializing userland...");
//
//     let userland_start = unsafe { USERLAND_START.as_ptr() as usize };
//     let userland_end = unsafe { USERLAND_END.as_ptr() as usize };
//     let userland_size = userland_end - userland_start;
//     let required_pages = userland_size.div_ceil(memory::PAGE_SIZE);
//     let userland_flags = EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::USER_ACCESSIBLE;
//
//     log::info!("  * code size: {userland_size} bytes, requires {required_pages} page(s)");
//
//     let mut page_table = PhysicalPageTable::new();
//     let mapper = page_table.mapper_mut();
//
//     log::info!(
//         "  * mapping userland to virtual address {:#010x}...",
//         USERLAND_VIRT_ADDR.0
//     );
//
//     for frame_idx in 0..required_pages {
//         let page = VirtualAddress(USERLAND_VIRT_ADDR.0 + (frame_idx * memory::PAGE_SIZE) as u64);
//         mapper.map(page, userland_flags, allocator);
//     }
//
//     log::info!("  * copying userland code to allocated memory");
//     unsafe {
//         core::ptr::copy_nonoverlapping(
//             userland_start as *mut u8,
//             USERLAND_VIRT_ADDR.0 as _,
//             userland_size,
//         );
//     }
// }
//
// pub fn get_user_fn_address(original_fn: extern "C" fn()) -> VirtualAddress {
//     let addr = original_fn as usize;
//     let offset = addr - unsafe { USERLAND_START.as_ptr() as usize };
//
//     VirtualAddress(USERLAND_VIRT_ADDR.0 + offset as u64)
// }
