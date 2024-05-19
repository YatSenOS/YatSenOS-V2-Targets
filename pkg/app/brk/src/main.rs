#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use lib::*;

extern crate lib;

fn main() -> usize {
    println!("Welcome to Brk Test!");
    loop {
        print!("$ ");
        let command = stdin().read_line();
        let line: Vec<&str> = command.trim().split(' ').collect();

        match line[0] {
            "brk" => {
                if line.len() != 2 {
                    println!("Usage: brk <addr>");
                    continue;
                }

                let addr = if line[1].starts_with("0x") {
                    usize::from_str_radix(&line[1][2..], 16)
                } else {
                    usize::from_str_radix(line[1], 10)
                };

                let addr = match addr {
                    Ok(addr) => addr,
                    Err(_) => {
                        println!("Invalid address: {}", line[1]);
                        continue;
                    }
                };

                match sys_brk(Some(addr)) {
                    Some(new_brk) => {
                        println!("Brk to {:#x} success, new brk addr: {:#x}", addr, new_brk)
                    }
                    None => println!("Brk to {:#x} failed", addr),
                }

                sys_stat();
            }
            "cur" => match sys_brk(None) {
                Some(brk) => println!("Current brk addr: {:#x}", brk),
                None => println!("Failed to get current brk addr"),
            },
            "exit" => {
                break;
            }
            _ => {
                println!("Unknown command: {}", line[0]);
            }
        }
    }

    0
}

entry!(main);
