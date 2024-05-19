#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use core::arch::asm;
use uefi::prelude::*;
use x86_64::registers::control::*;
use x86_64::structures::paging::*;
use x86_64::VirtAddr;
use xmas_elf::ElfFile;
use ysos_boot::allocator::*;
use ysos_boot::fs::*;
use ysos_boot::BootInfo;
use ysos_boot::MemoryType;

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";

#[entry]
fn efi_main(image: uefi::Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);
    info!("Running UEFI bootloader...");

    // 1. Load config
    let bs = system_table.boot_services();
    let config = {
        let mut file = open_file(bs, CONFIG_PATH);
        let buf = load_file(bs, &mut file);
        config::Config::parse(buf)
    };

    info!("config: {:#x?}", config);

    // 2. Load ELF files
    let elf = {
        let mut file = open_file(bs, config.kernel_path);
        let buf = load_file(bs, &mut file);
        ElfFile::new(buf).expect("failed to parse ELF")
    };
    unsafe {
        ENTRY = elf.header.pt2.entry_point() as usize;
    }

    let apps = if config.load_apps {
        info!("Loading apps...");
        Some(load_apps(system_table.boot_services()))
    } else {
        info!("Skip loading apps");
        None
    };

    // 3. Load MemoryMap
    let max_mmap_size = system_table.boot_services().memory_map_size();
    let mmap_storage = Box::leak(
        vec![0; max_mmap_size.map_size + 10 * max_mmap_size.entry_size].into_boxed_slice(),
    );
    let mmap = system_table
        .boot_services()
        .memory_map(mmap_storage)
        .expect("Failed to get memory map");

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
        &mut UEFIFrameAllocator(bs),
    );

    elf::load_elf(
        &elf,
        config.physical_memory_offset,
        &mut page_table,
        &mut UEFIFrameAllocator(bs),
        false,
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
        &mut UEFIFrameAllocator(bs),
        false,
    )
    .expect("Failed to map stack");

    // recover write protect
    unsafe {
        Cr0::update(|f| f.insert(Cr0Flags::WRITE_PROTECT));
    }

    free_elf(bs, elf);

    // 5. Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let (runtime, mmap) = system_table.exit_boot_services(MemoryType::LOADER_DATA);
    // NOTE: alloc & log can no longer be used

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
        system_table: runtime,
        loaded_apps: apps,
        log_level: config.log_level,
    };

    // align stack to 8 bytes
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;

    unsafe {
        jump_to_entry(&bootinfo, stacktop);
    }
}

/// Get current page table from CR3
fn current_page_table() -> OffsetPageTable<'static> {
    let p4_table_addr = Cr3::read().0.start_address().as_u64();
    let p4_table = unsafe { &mut *(p4_table_addr as *mut PageTable) };
    unsafe { OffsetPageTable::new(p4_table, VirtAddr::new(0)) }
}

/// The entry point of kernel, set by BSP.
static mut ENTRY: usize = 0;

/// Jump to ELF entry according to global variable `ENTRY`
#[allow(clippy::empty_loop)]
unsafe fn jump_to_entry(bootinfo: *const BootInfo, stacktop: u64) -> ! {
    asm!("mov rsp, {}; call {}", in(reg) stacktop, in(reg) ENTRY, in("rdi") bootinfo);
    loop {}
}
