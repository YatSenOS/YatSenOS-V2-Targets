#![no_std]
#![no_main]

use alloc::string::ToString;
use log::*;
use ysos::*;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    let mut executor = Executor::new();

    // use executor.spawn() to spawn kernel tasks
    executor.run(spawn_init(boot_info));
    ysos::shutdown(boot_info);
}

pub fn spawn_init(boot_info: &'static boot::BootInfo) -> process::ProcessId {
    // print_serial!("\x1b[1;1H\x1b[2J");

    if let Some(apps) = &boot_info.loaded_apps {
        for app in apps {
            if app.name.eq("sh") {
                info!("Found sh in loaded apps, spawning...");
                return process::elf_spawn("sh".to_string(), &app.elf, None).unwrap();
            }
        }
    }

    let sh_file = filesystem::try_get_file("/APP/SH", fs::Mode::ReadOnly).unwrap();
    process::spawn(&sh_file).unwrap()
}
