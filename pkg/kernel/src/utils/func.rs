pub fn test() -> ! {
    let mut count = 0;
    let id;
    if let Some(id_env) = crate::process::env("id") {
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

#[inline(never)]
fn huge_stack() {
    println!("Huge stack testing...");

    let mut stack = [0u64; 0x1000];

    for (idx, item) in stack.iter_mut().enumerate() {
        *item = idx as u64;
    }

    for i in 0..stack.len() / 256 {
        println!("{:#05x} == {:#05x}", i * 256, stack[i * 256]);
    }
}

pub fn stack_test() -> ! {
    huge_stack();
    kill_self();
}

pub fn kill_self() -> ! {
    crate::process::process_exit(0);

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
