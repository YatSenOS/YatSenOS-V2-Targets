#![no_std]
#![no_main]

use ysos::*;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    let mut test_num = 0;

    loop {
        print!("[>] ");
        let line = input::get_line();
        match line.trim() {
            "exit" => break,
            "ps" => {
                ysos::proc::print_process_list();
            }
            "stack" => {
                ysos::stack_thread_test();
            }
            "test" => {
                ysos::new_test_thread(format!("{}", test_num).as_str());
                test_num += 1;
            }
            _ => println!("[=] {}", line),
        }
    }

    ysos::shutdown(boot_info);
}
