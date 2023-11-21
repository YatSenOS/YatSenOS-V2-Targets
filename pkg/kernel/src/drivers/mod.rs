mod uart16550;

pub mod ata;
pub mod filesystem;
pub mod input;
pub mod serial;

pub use input::{get_key, push_key};
