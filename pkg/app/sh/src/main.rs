#![no_std]
#![no_main]

extern crate alloc;

mod services;
mod utils;

use alloc::string::ToString;
use alloc::vec::Vec;
use lib::*;
use owo_colors::OwoColorize;

extern crate lib;

fn main() -> isize {
    utils::show_welcome_text();
    loop {
        print!("{} ", "$".cyan().bold());
        let input = stdin().read_line();
        let line: Vec<&str> = input.trim().split(' ').collect();
        match line[0] {
            "\x04" | "exit" => {
                println!();
                break;
            }
            "ps" => sys_stat(),
            "ls" => sys_list_app(),
            "exec" => {
                if line.len() < 2 {
                    println!("Usage: exec <file>");
                    continue;
                }

                services::exec(line[1]);
            }
            "kill" => {
                if line.len() < 2 {
                    println!("Usage: kill <pid>");
                    continue;
                }
                let pid = line[1].to_string().parse::<u16>();

                if pid.is_err() {
                    errln!("Cannot parse pid");
                    continue;
                }

                services::kill(pid.unwrap());
            }
            "help" => utils::show_help_text(),
            "clear" => print!("\x1b[1;1H\x1b[2J"),
            _ => {
                if line[0].is_empty() {
                    println!();
                    continue;
                }
                println!("[=] you said \"{}\"", input)
            }
        }
    }

    0
}

entry!(main);
