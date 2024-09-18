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

    fn try_read_key_with_buf(&self, buf: &mut [u8]) -> Option<u8> {
        if let Some(bytes) = sys_read(0, buf) {
            if bytes == 1 {
                return Some(buf[0]);
            }
        }
        None
    }

    fn pop_key(&self) -> u8 {
        let mut buf = [0];
        loop {
            if let Some(key) = self.try_read_key_with_buf(&mut buf) {
                return key;
            }
        }
    }

    pub fn read_line(&self) -> String {
        let mut string = String::new();
        loop {
            let ch = self.pop_key();

            match ch {
                0x0d => {
                    stdout().write("\n");
                    break;
                }
                0x03 => {
                    string.clear();
                    break;
                }
                0x08 | 0x7F if !string.is_empty() => {
                    stdout().write("\x08 \x08");
                    string.pop();
                }
                _ => {
                    if Self::is_utf8(ch) {
                        let utf_char = char::from_u32(self.to_utf8(ch)).unwrap();
                        string.push(utf_char);
                        print! {"{}", utf_char};
                    } else {
                        string.push(ch as char);
                        print!("{}", ch as char);
                    }
                }
            }
        }
        string
    }

    fn is_utf8(ch: u8) -> bool {
        ch & 0x80 == 0 || ch & 0xE0 == 0xC0 || ch & 0xF0 == 0xE0 || ch & 0xF8 == 0xF0
    }

    fn to_utf8(&self, ch: u8) -> u32 {
        let mut codepoint = 0;

        if ch & 0x80 == 0 {
            codepoint = ch as u32;
        } else if ch & 0xE0 == 0xC0 {
            codepoint = ((ch & 0x1F) as u32) << 6;
            codepoint |= (self.pop_key() & 0x3F) as u32;
        } else if ch & 0xF0 == 0xE0 {
            codepoint = ((ch & 0x0F) as u32) << 12;
            codepoint |= ((self.pop_key() & 0x3F) as u32) << 6;
            codepoint |= (self.pop_key() & 0x3F) as u32;
        } else if ch & 0xF8 == 0xF0 {
            codepoint = ((ch & 0x07) as u32) << 18;
            codepoint |= ((self.pop_key() & 0x3F) as u32) << 12;
            codepoint |= ((self.pop_key() & 0x3F) as u32) << 6;
            codepoint |= (self.pop_key() & 0x3F) as u32;
        }

        codepoint
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
