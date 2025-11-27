#![no_std]
#![no_main]

extern crate alloc;

mod services;
mod utils;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use lib::*;
use owo_colors::OwoColorize;

extern crate lib;

fn main() -> isize {
    let mut root_dir = String::from("/APP/");
    utils::show_welcome_text();
    loop {
        print!(
            "{} {} ",
            format_args!("[{}]", root_dir).bright_yellow().bold(),
            "$".cyan().bold()
        );
        let input = stdin().read_line();
        let line: Vec<&str> = input.trim().split(' ').collect();
        match line[0] {
            "\x04" | "exit" => {
                println!();
                break;
            }
            "ps" => sys_stat(),
            "ls" => sys_list_dir(root_dir.as_str()),
            "cat" => {
                if line.len() < 2 {
                    println!("Usage: cat <file>");
                    continue;
                }

                services::cat(line[1], root_dir.as_str());
            }
            "cd" => {
                if line.len() < 2 {
                    println!("Usage: cd <dir>");
                    continue;
                }

                services::cd(line[1], &mut root_dir);
            }
            "exec" => {
                if line.len() < 2 {
                    println!("Usage: exec <file>");
                    continue;
                }

                services::exec(line[1], root_dir.as_str());
            }
            "nohup" => {
                if line.len() < 2 {
                    println!("Usage: nohup <file>");
                    continue;
                }

                services::nohup(line[1], root_dir.as_str());
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
            "rand" => {
                let len = if line.len() < 2 {
                    16
                } else {
                    line[1].parse::<usize>().unwrap_or(16)
                };

                services::gen_random_bytes(len);
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
