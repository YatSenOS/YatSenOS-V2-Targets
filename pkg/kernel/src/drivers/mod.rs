mod uart16550;

pub mod input;
pub mod serial;

pub use input::{get_line, push_key};
