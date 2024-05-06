use alloc::string::String;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;

type Key = u8;
lazy_static! {
    static ref INPUT_BUF: ArrayQueue<Key> = ArrayQueue::new(128);
}

pub fn push_key(key: Key) {
    if INPUT_BUF.push(key).is_err() {
        warn!("Input buffer is full. Dropping key '{:?}'", key);
    }
}

#[inline]
pub fn try_pop_key() -> Option<Key> {
    INPUT_BUF.pop()
}

pub fn pop_key() -> u8 {
    loop {
        if let Some(data) = try_pop_key() {
            return data;
        }
    }
}

pub fn get_line() -> String {
    let mut line = String::with_capacity(256);
    loop {
        let ch = pop_key();

        match ch {
            13 => {
                println!();
                return line;
            }
            0x08 | 0x7F if !line.is_empty() => {
                print!("\x08\x20\x08");
                line.pop();
            }
            _ => {
                if is_utf8(ch) {
                    let utf_char = char::from_u32(to_utf8(ch)).unwrap();
                    line.push(utf_char);
                    print! {"{}", utf_char};
                } else {
                    line.push(ch as char);
                    print!("{}", ch as char);
                }
            }
        }
    }
}

pub fn is_utf8(ch: u8) -> bool {
    ch & 0x80 == 0 || ch & 0xE0 == 0xC0 || ch & 0xF0 == 0xE0 || ch & 0xF8 == 0xF0
}

pub fn to_utf8(ch: u8) -> u32 {
    let mut codepoint = 0;

    if ch & 0x80 == 0 {
        codepoint = ch as u32;
    } else if ch & 0xE0 == 0xC0 {
        codepoint = ((ch & 0x1F) as u32) << 6;
        codepoint |= (pop_key() & 0x3F) as u32;
    } else if ch & 0xF0 == 0xE0 {
        codepoint = ((ch & 0x0F) as u32) << 12;
        codepoint |= ((pop_key() & 0x3F) as u32) << 6;
        codepoint |= (pop_key() & 0x3F) as u32;
    } else if ch & 0xF8 == 0xF0 {
        codepoint = ((ch & 0x07) as u32) << 18;
        codepoint |= ((pop_key() & 0x3F) as u32) << 12;
        codepoint |= ((pop_key() & 0x3F) as u32) << 6;
        codepoint |= (pop_key() & 0x3F) as u32;
    }

    codepoint
}
