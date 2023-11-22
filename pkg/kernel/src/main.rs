#![no_std]
#![no_main]

use ysos::*;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    let mut executor = Executor::new();

    // use executor.spawn() to spawn kernel tasks
    executor.run(spawn_init());
    ysos::shutdown(boot_info);
}

pub fn spawn_init() -> process::ProcessId {
    print_serial!("\x1b[1;1H\x1b[2J");
    process::spawn(&"sh").unwrap()
}
