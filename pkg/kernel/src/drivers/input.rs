use crate::drivers::serial;
use alloc::string::String;
use crossbeam_queue::ArrayQueue;
use pc_keyboard::DecodedKey;
use x86_64::instructions::interrupts;

once_mutex!(pub INPUT_BUF: ArrayQueue<DecodedKey>);

const DEFAULT_BUF_SIZE: usize = 128;

guard_access_fn!(pub get_input_buf(INPUT_BUF: ArrayQueue<DecodedKey>));

pub fn init() {
    init_INPUT_BUF(ArrayQueue::new(DEFAULT_BUF_SIZE));
    info!("Input Initialized.");
}

pub fn push_key(key: DecodedKey) {
    if let Some(queue) = get_input_buf() {
        if queue.push(key).is_err() {
            warn!("Input buffer is full. Dropping key '{:?}'", key);
        }
    }
}

pub fn try_get_key() -> Option<DecodedKey> {
    interrupts::without_interrupts(|| get_input_buf_for_sure().pop())
}

pub fn get_key() -> DecodedKey {
    loop {
        if let Some(k) = try_get_key() {
            return k;
        }
    }
}

pub fn get_line() -> String {
    let mut s = String::with_capacity(DEFAULT_BUF_SIZE);
    loop {
        let key = get_key();
        if let DecodedKey::Unicode(k) = key {
            match k {
                '\n' => break,
                '\x08' => {
                    if !s.is_empty() {
                        serial::backspace();
                        s.pop(); // remove previous char
                    }
                }
                c => {
                    print!("{}", k);
                    s.push(c)
                }
            }
        }
    }
    println!();
    s
}