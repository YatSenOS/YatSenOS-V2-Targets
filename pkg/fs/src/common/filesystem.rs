//! The filesystem trait definitions needed to implement new virtual filesystems
use alloc::string::String;
use alloc::boxed::Box;
use super::*;

use core::fmt::Debug;

/// File system trait
pub trait FileSystem: Debug + Sync + Send {
    /// Iterates over all direct children of this directory path
    fn read_dir(&self, path: &str) -> Result<Box<dyn Iterator<Item = FsMetadata> + Send>>;

    /// Opens the file at this path for reading
    fn open_file(&self, path: &str) -> Result<Box<dyn SeekAndRead + Send>>;

    /// Returns the file metadata for the file at this path
    fn metadata(&self, path: &str) -> Result<FsMetadata>;

    /// Returns true if a file or directory at path exists, false otherwise
    fn exists(&self, path: &str) -> Result<bool>;
}
