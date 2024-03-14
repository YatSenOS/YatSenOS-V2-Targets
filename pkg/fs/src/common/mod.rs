#[macro_use]
mod macros;

mod io;
mod block;
mod device;
mod error;
mod path;
mod metadata;
mod filesystem;

use super::*;
pub use io::*; // Done
pub use error::*; // Done
pub use device::*; // Done
pub use block::*; // Done
pub use path::*;
pub use metadata::*; // Done
pub use filesystem::*;

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
