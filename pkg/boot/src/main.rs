#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use uefi::mem::memory_map::MemoryMap;
use uefi::{Status, entry};
use x86_64::VirtAddr;
use x86_64::registers::control::*;
use x86_64::structures::paging::*;
use xmas_elf::ElfFile;
use ysos_boot::allocator::*;
use ysos_boot::fs::*;
use ysos_boot::*;

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);
    info!("Running UEFI bootloader...");

    // 1. Load config
    let config = {
        let mut file = open_file(CONFIG_PATH);
        let buf = load_file(&mut file);
        config::Config::parse(buf)
    };

    info!("config: {:#x?}", config);

    // 2. Load ELF files
    let elf = {
        let mut file = open_file(config.kernel_path);
        let buf = load_file(&mut file);
        ElfFile::new(buf).expect("failed to parse ELF")
    };

    set_entry(elf.header.pt2.entry_point() as usize);

    // 3. Load MemoryMap
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA).expect("Failed to get memory map");

    let max_phys_addr = mmap
        .entries()
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area

    // 4. Map ELF segments, kernel stack and physical memory to virtual memory
    let mut page_table = current_page_table();

    // root page table is readonly, disable write protect
    unsafe {
        Cr0::update(|f| f.remove(Cr0Flags::WRITE_PROTECT));
        Efer::update(|f| f.insert(EferFlags::NO_EXECUTE_ENABLE));
    }

    elf::map_physical_memory(
        config.physical_memory_offset,
        max_phys_addr,
        &mut page_table,
        &mut UEFIFrameAllocator,
    );

    elf::load_elf(
        &elf,
        config.physical_memory_offset,
        &mut page_table,
        &mut UEFIFrameAllocator,
    )
    .expect("Failed to load ELF");

    let (stack_start, stack_size) = if config.kernel_stack_auto_grow > 0 {
        let stack_start = config.kernel_stack_address
            + (config.kernel_stack_size - config.kernel_stack_auto_grow) * 0x1000;
        (stack_start, config.kernel_stack_auto_grow)
    } else {
        (config.kernel_stack_address, config.kernel_stack_size)
    };

    info!(
        "Kernel init stack: [0x{:x?} -> 0x{:x?})",
        stack_start,
        stack_start + stack_size * 0x1000
    );

    elf::map_pages(
        stack_start,
        stack_size,
        &mut page_table,
        &mut UEFIFrameAllocator,
    )
    .expect("Failed to map stack");

    // recover write protect
    unsafe {
        Cr0::update(|f| f.insert(Cr0Flags::WRITE_PROTECT));
    }

    free_elf(elf);

    // 5. Pass system table to kernel
    let ptr = uefi::table::system_table_raw().expect("Failed to get system table");
    let system_table = ptr.cast::<core::ffi::c_void>();

    // 6. Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let mmap = unsafe { uefi::boot::exit_boot_services(MemoryType::LOADER_DATA) };
    // NOTE: alloc & log can no longer be used

    // 7. Construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
        log_level: config.log_level,
        system_table,
    };

    // align stack to 8 bytes
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;

    jump_to_entry(&bootinfo, stacktop);
}

/// Get current page table from CR3
fn current_page_table() -> OffsetPageTable<'static> {
    let p4_table_addr = Cr3::read().0.start_address().as_u64();
    let p4_table = unsafe { &mut *(p4_table_addr as *mut PageTable) };
    unsafe { OffsetPageTable::new(p4_table, VirtAddr::new(0)) }
}
