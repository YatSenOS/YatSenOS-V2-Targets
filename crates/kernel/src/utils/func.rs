use alloc::vec::Vec;

use storage::FileSystem;

use crate::filesystem::get_rootfs;

pub fn test() -> ! {
    let mut count = 0;
    let id;
    if let Some(id_env) = crate::proc::env("id") {
        id = id_env
    } else {
        id = "unknown".into()
    }
    loop {
        count += 1;
        if count == 100 {
            count = 0;
            print_serial!("\r{:-6} => Hello, world!", id);
        }
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn load_file_test() {
    let mut handle = get_rootfs().open_file("/APP/SH").unwrap();

    let mut file_buffer = Vec::new();

    let start = unsafe { core::arch::x86_64::_rdtsc() };

    handle.read_all(&mut file_buffer).ok();

    let end = unsafe { core::arch::x86_64::_rdtsc() };

    let diff = end - start;

    debug!("Load test: {}", diff);
}
