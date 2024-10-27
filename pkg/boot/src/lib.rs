#![no_std]
pub use uefi::data_types::chars::*;
pub use uefi::data_types::*;
pub use uefi::proto::console::gop::{GraphicsOutput, ModeInfo};
pub use uefi::boot::{MemoryAttribute, MemoryDescriptor, MemoryType};
pub use uefi::Status;

use arrayvec::ArrayVec;
use core::ptr::NonNull;

pub mod allocator;
pub mod config;
pub mod fs;

#[macro_use]
extern crate log;

pub type MemoryMap = ArrayVec<MemoryDescriptor, 256>;

/// This structure represents the information that the bootloader passes to the kernel.
pub struct BootInfo {
    /// The memory map
    pub memory_map: MemoryMap,

    /// The offset into the virtual address space where the physical memory is mapped.
    pub physical_memory_offset: u64,

    /// The system table virtual address
    pub system_table: NonNull<core::ffi::c_void>,
}

/// This is copied from https://docs.rs/bootloader/0.10.12/src/bootloader/lib.rs.html
/// Defines the entry point function.
///
/// The function must have the signature `fn(&'static BootInfo) -> !`.
///
/// This macro just creates a function named `_start`, which the linker will use as the entry
/// point. The advantage of using this macro instead of providing an own `_start` function is
/// that the macro ensures that the function and argument types are correct.
#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[export_name = "_start"]
        pub extern "C" fn __impl_start(boot_info: &'static $crate::BootInfo) -> ! {
            // validate the signature of the program entry point
            let f: fn(&'static $crate::BootInfo) -> ! = $path;

            f(boot_info)
        }
    };
}
