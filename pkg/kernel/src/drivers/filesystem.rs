use crate::ata::*;
use alloc::boxed::Box;
use chrono::DateTime;
use storage::fat16::Fat16;
use storage::mbr::*;
use storage::*;

pub static ROOTFS: spin::Once<Mount> = spin::Once::new();

pub fn get_rootfs() -> &'static Mount {
    ROOTFS.get().unwrap()
}

#[derive(Debug, Clone)]
pub enum StdIO {
    Stdin,
    Stdout,
    Stderr,
}

pub fn init() {
    info!("Opening disk device...");

    let drive = Drive::open(0, 0).expect("Failed to open disk device");
    let mut parts = MbrTable::parse(drive)
        .expect("Failed to parse MBR")
        .partitions()
        .expect("Failed to get partitions");

    info!("Mounting filesystem...");
    ROOTFS.call_once(|| Mount::new(Box::new(Fat16::new(parts.remove(0))), "/".into()));

    trace!("Root filesystem: {:#?}", ROOTFS.get().unwrap());

    info!("Initialized Filesystem.");
}

pub fn ls(root_path: &str) {
    let iter = get_rootfs().read_dir(root_path);

    if let Err(err) = iter {
        warn!("{:?}", err);
        return;
    }

    let iter = iter.unwrap();

    println!("  Size | Last Modified       | Name");

    for meta in iter {
        let (size, unit) = crate::humanized_size_short(meta.len as u64);
        println!(
            "{:>5.*}{} | {} | {}{}",
            1,
            size,
            unit,
            meta.modified
                .map(|t| t.format("%Y/%m/%d %H:%M:%S"))
                .unwrap_or(
                    DateTime::from_timestamp_millis(0)
                        .unwrap()
                        .format("%Y/%m/%d %H:%M:%S")
                ),
            meta.name,
            if meta.is_dir() { "/" } else { "" }
        );
    }
}
