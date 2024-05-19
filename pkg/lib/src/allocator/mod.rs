#[cfg(feature = "kernel_alloc")]
mod kernel;

#[cfg(feature = "kernel_alloc")]
pub use kernel::*;
