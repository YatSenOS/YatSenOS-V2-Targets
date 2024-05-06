#![no_std]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(type_alias_impl_trait)]
#![feature(panic_info_message)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate bitflags;
extern crate libm;

#[macro_use]
pub mod utils;
pub use utils::*;

#[macro_use]
pub mod drivers;
pub use drivers::*;

pub mod interrupt;
pub mod memory;
pub mod proc;

pub use alloc::format;
use boot::BootInfo;

pub fn init(boot_info: &'static BootInfo) {
    serial::init(); // init serial output
    logger::init(boot_info); // init logger system
    memory::address::init(boot_info);
    memory::gdt::init(); // init gdt
    memory::allocator::init(); // init kernel heap allocator
    interrupt::init(); // init interrupts
    clock::init(boot_info); // init clock (uefi service)
    memory::init(boot_info); // init memory manager
    memory::user::init(); // init user heap allocator
    proc::init(boot_info); // init task manager

    x86_64::instructions::interrupts::enable();
    info!("Interrupts Enabled.");

    info!("YatSenOS initialized.");
}

pub fn wait(init: proc::ProcessId) {
    loop {
        if proc::wait_no_block(init).is_none() {
            x86_64::instructions::hlt();
        } else {
            break;
        }
    }
}

pub fn shutdown(boot_info: &'static BootInfo) -> ! {
    info!("YatSenOS shutting down.");
    unsafe {
        boot_info.system_table.runtime_services().reset(
            boot::ResetType::SHUTDOWN,
            boot::UefiStatus::SUCCESS,
            None,
        );
    }
}
