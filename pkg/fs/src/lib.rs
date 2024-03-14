#![cfg_attr(not(test), no_std)]
#![allow(dead_code, unused_imports)]
#![feature(trait_alias)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;

pub mod device;
pub mod structs;

use alloc::vec::Vec;
use structs::dir_entry::Cluster;

pub use device::*;
pub use file::Mode;
pub use structs::*;

// 1. The disk structure
// How to read a file from disk
//
//   - The disk is a collection of partitions.
//     MBR (Master Boot Record) is the first sector of the disk.
//     The MBR contains information about the partitions.
//
//     [ MBR | Partitions ] [ Partition 1 ] [ Partition 2 ] [ Partition 3 ] [ Partition 4 ]
//
// 2. The partition structure (in FAT16)
//
//    - The partition is a collection of clusters.
//     BPB (Boot Parameter Block) is the first sector of the partition.
//     The BPB contains information about the filesystem.
//
//     [ FAT16 BPB ] [ Data ]

pub fn root_dir() -> Directory {
    Directory::new(Cluster::ROOT_DIR)
}

/// Call a callback function for each directory entry in a directory.
pub fn iterate_dir<T, F>(
    volume: &FAT16Volume<T>,
    dir: &Directory,
    func: F,
) -> Result<(), VolumeError>
where
    T: BlockDevice,
    F: FnMut(&DirEntry),
{
    volume.iterate_dir(dir, func)
}

/// Look in a directory for a named file.
pub fn find_directory_entry<T>(
    volume: &FAT16Volume<T>,
    dir: &Directory,
    name: &str,
) -> Result<Directory, VolumeError>
where
    T: BlockDevice,
{
    if name.len() < 2 {
        return Ok(root_dir());
    }

    let res = volume.find_directory_entry(dir, name)?;

    if res.is_directory() {
        Ok(Directory::from_entry(res))
    } else {
        Err(VolumeError::NotADirectory)
    }
}

/// Open a dir in a dir
pub fn open_dir<T>(
    volume: &FAT16Volume<T>,
    parent: &Directory,
    name: &str,
) -> Result<Directory, VolumeError>
where
    T: BlockDevice,
{
    let dir_entry = volume.find_directory_entry(parent, name)?;

    if !dir_entry.is_directory() {
        return Err(VolumeError::NotADirectory);
    }

    let dir = Directory::from_entry(dir_entry);

    trace!("Opened dir: {:#?}", &dir);

    Ok(dir)
}

/// Open a file in a dir
pub fn open_file<T>(
    volume: &FAT16Volume<T>,
    parent: &Directory,
    name: &str,
    mode: Mode,
) -> Result<File, VolumeError>
where
    T: BlockDevice,
{
    trace!("Try open file: {}", name);
    let dir_entry = volume.find_directory_entry(parent, name)?;

    if dir_entry.is_directory() {
        return Err(VolumeError::NotAFile);
    }

    if dir_entry.is_readonly() && mode != Mode::ReadOnly {
        return Err(VolumeError::ReadOnly);
    }

    let file = match mode {
        Mode::ReadOnly => File {
            start_cluster: dir_entry.cluster,
            length: dir_entry.size,
            mode,
            entry: dir_entry,
        },
        _ => return Err(VolumeError::Unsupported),
    };

    trace!("Opened file: {:#?}", &file);

    Ok(file)
}

/// read a file
pub fn read<T>(volume: &FAT16Volume<T>, file: &File) -> Result<Vec<u8>, VolumeError>
where
    T: BlockDevice,
{
    let mut data = vec![0u8; file.length() as usize];
    let mut length = file.length() as usize;
    let mut block = Block::default();

    for i in 0..=file.length() as usize / Block::SIZE {
        let sector = volume.cluster_to_sector(&file.start_cluster());
        volume.read_block(sector + i, &mut block).unwrap();
        if length > Block::SIZE {
            data[i * Block::SIZE..(i + 1) * Block::SIZE].copy_from_slice(block.as_u8_slice());
            length -= Block::SIZE;
        } else {
            data[i * Block::SIZE..i * Block::SIZE + length].copy_from_slice(&block[..length]);
            break;
        }
    }
    Ok(data)
}

/// read a file
pub fn read_to_buf<T>(
    volume: &FAT16Volume<T>,
    file: &File,
    buf: &mut [u8],
    mut offset: usize,
) -> Result<usize, DeviceError>
where
    T: BlockDevice,
{
    let length = file.length() as usize;

    if offset >= length {
        return Ok(0);
    }

    let total_blocks = (length + Block::SIZE - 1) / Block::SIZE;
    let mut current_block = offset / Block::SIZE;
    let mut block = Block::default();
    let sector = volume.cluster_to_sector(&file.start_cluster());

    let mut bytes_read = 0;

    while bytes_read < buf.len() && offset < length && current_block < total_blocks {
        current_block = offset / Block::SIZE;
        let current_offset = offset % Block::SIZE;
        volume.read_block(sector + current_block, &mut block)?;

        let block_remain = Block::SIZE - current_offset;
        let buf_remain = buf.len() - bytes_read;
        let file_remain = length - offset;
        let to_read = buf_remain.min(block_remain).min(file_remain);

        buf[bytes_read..bytes_read + to_read]
            .copy_from_slice(&block[current_offset..current_offset + to_read]);

        bytes_read += to_read;
        offset += to_read;
    }

    Ok(bytes_read)
}
