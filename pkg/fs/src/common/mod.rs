#[macro_use]
mod macros;

mod block;
mod device;
mod error;
mod filehandle;
mod filesystem;
mod io;
mod metadata;
mod path;

use super::*;
pub use block::*; // Done
pub use device::*; // Done
pub use error::*; // Done
pub use filehandle::*;
pub use filesystem::*;
pub use io::*; // Done
pub use metadata::*; // Done
pub use path::*;

pub fn humanized_size(size: usize) -> (f32, String) {
    let bytes = size as f32;
    if bytes < 1024f32 {
        (bytes, String::from("B"))
    } else if (bytes / (1 << 10) as f32) < 1024f32 {
        (bytes / (1 << 10) as f32, String::from("K"))
    } else if (bytes / (1 << 20) as f32) < 1024f32 {
        (bytes / (1 << 20) as f32, String::from("M"))
    } else {
        (bytes / (1 << 30) as f32, String::from("G"))
    }
}
