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

    pub fn read_char(&self) -> Option<char> {
        let mut buf = vec![0; 4];
        if let Some(bytes) = sys_read(0, &mut buf) {
            if bytes > 0 {
                return Some(String::from_utf8_lossy(&buf[..bytes]).to_string().remove(0));
            }
        }
        None
    }

    pub fn read_char_with_buf(&self, buf: &mut [u8]) -> Option<char> {
        if let Some(bytes) = sys_read(0, buf) {
            if bytes > 0 {
                return Some(String::from_utf8_lossy(&buf[..bytes]).to_string().remove(0));
            }
        }
        None
    }

    pub fn read_line(&self) -> String {
        let mut string = String::new();
        let mut buf = vec![0; 4];
        loop {
            if let Some(k) = self.read_char_with_buf(&mut buf[..4]) {
                match k {
                    '\n' => {
                        stdout().write("\n");
                        break;
                    }
                    '\x03' => {
                        string.clear();
                        break;
                    }
                    '\x04' => {
                        string.clear();
                        string.push('\x04');
                        break;
                    }
                    '\x08' => {
                        if !string.is_empty() {
                            stdout().write("\x08");
                            string.pop();
                        }
                    }
                    // ignore other control characters
                    '\x00'..='\x1F' => {}
                    c => {
                        self::print!("{}", k);
                        string.push(c);
                    }
                }
            }
        }
        string
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

/// The different ways we can open a file.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum FileMode {
    /// Open a file for reading, if it exists.
    ReadOnly = 0,
    /// Open a file for appending (writing to the end of the existing file), if it exists.
    ReadWriteAppend = 1,
    /// Open a file and remove all contents, before writing to the start of the existing file, if it exists.
    ReadWriteTruncate = 2,
    /// Create a new empty file. Fail if it exists.
    ReadWriteCreate = 3,
    /// Create a new empty file, or truncate an existing file.
    ReadWriteCreateOrTruncate = 4,
    /// Create a new empty file, or append to an existing file.
    ReadWriteCreateOrAppend = 5,
}
