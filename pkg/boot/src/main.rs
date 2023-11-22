#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use core::arch::asm;
use uefi::prelude::*;

#[entry]
fn efi_main(image: uefi::Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).expect("Failed to initialize utilities");
    log::set_max_level(log::LevelFilter::Info);

    loop {
        info!("Hello World from UEFI bootloader!");

        for _ in 0..0x10000000 {
            unsafe {
                asm!("nop");
            }
        }
    }
}
