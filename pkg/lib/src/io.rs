use crate::*;
use alloc::string::{String, ToString};
use alloc::vec;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    fn new() -> Self {
        Self
    }

    pub fn read_line(&self) -> String {
        // allocate string
        let mut line = String::new();

        // read from input buffer char by char
        loop {
            let buf: &mut [u8] = &mut [0u8; 256];
            let ret = sys_read(0, buf);

            if ret.is_none() {
                continue;
            } else {
                for i in 0..ret.unwrap() {
                    let c = buf[i];
                    // handle backspace / enter... and finally return the string
                    match c {
                        13 => {
                            sys_write(1, "\n".as_bytes());
                            return line;
                        }
                        0x08 | 0x7F => {
                            line.pop();
                            sys_write(1, "\x08\x20\x08".as_bytes());
                        }
                        _ => {
                            line.push(c as char);
                            sys_write(1, &mut [c]);
                        }
                    };
                }
            }
        }
    }
}

impl Stdout {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(1, s.as_bytes());
    }
}

impl Stderr {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(2, s.as_bytes());
    }
}

pub fn stdin() -> Stdin {
    Stdin::new()
}

pub fn stdout() -> Stdout {
    Stdout::new()
}

pub fn stderr() -> Stderr {
    Stderr::new()
}
