#![allow(clippy::len_without_is_empty)]

use alloc::{collections::BTreeMap, string::String};
use fs::{Device, File, Random};
use pc_keyboard::DecodedKey;
use spin::Mutex;

use crate::{
    filesystem::{get_volume, StdIO},
    input::try_get_key,
};

#[derive(Debug)]
pub struct ResourceSet {
    pub handles: BTreeMap<u8, Mutex<ResourceHandle>>,
}

impl Default for ResourceSet {
    fn default() -> Self {
        let mut res = Self {
            handles: BTreeMap::new(),
        };

        res.open(Resource::Console(StdIO::Stdin));
        res.open(Resource::Console(StdIO::Stdout));
        res.open(Resource::Console(StdIO::Stderr));

        res
    }
}

impl ResourceSet {
    pub fn open(&mut self, res: Resource) -> u8 {
        let fd = self.handles.len() as u8;
        self.handles
            .insert(fd, Mutex::new(ResourceHandle::new(res)));
        fd
    }

    pub fn close(&mut self, fd: u8) -> bool {
        self.handles.remove(&fd).is_some()
    }

    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        if let Some(handle) = self.handles.get(&fd) {
            handle.lock().read(buf)
        } else {
            0
        }
    }

    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        if let Some(handle) = self.handles.get(&fd) {
            handle.lock().write(buf)
        } else {
            0
        }
    }

    pub fn seek(&self, fd: u8, offset: usize) {
        if let Some(handle) = self.handles.get(&fd) {
            handle.lock().seek(offset)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Resource {
    File(File),
    Console(StdIO),
    Random(Random),
    Null,
}

#[derive(Debug, Clone)]
pub struct ResourceHandle {
    pub resource: Resource,
    pub offset: usize,
}

impl ResourceHandle {
    pub fn new(resource: Resource) -> Self {
        Self {
            resource,
            offset: 0,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> isize {
        if let Some(bytes) = self.resource.read(buf, self.offset) {
            self.offset += bytes;
            bytes as isize
        } else {
            -1
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> isize {
        if let Some(bytes) = self.resource.write(buf, self.offset) {
            self.offset += bytes;
            bytes as isize
        } else {
            -1
        }
    }

    pub fn seek(&mut self, offset: usize) {
        self.offset = offset
    }

    pub fn len(&self) -> usize {
        self.resource.len()
    }
}

impl Resource {
    fn read(&self, buf: &mut [u8], offset: usize) -> Option<usize> {
        match self {
            Resource::File(file) => {
                let ret = fs::read_to_buf(get_volume(), file, buf, offset);
                if let Err(e) = ret {
                    error!("Failed to read file: {:?}", e);
                    None
                } else {
                    Some(ret.unwrap())
                }
            }
            Resource::Console(stdio) => match stdio {
                &StdIO::Stdin => {
                    return Some(if buf.len() < 4 {
                        0
                    } else if let Some(DecodedKey::Unicode(k)) = try_get_key() {
                        let s = k.encode_utf8(buf);
                        s.len()
                    } else {
                        0
                    });
                }
                _ => Some(0),
            },
            Resource::Random(random) => Some(random.read(buf, 0, buf.len()).unwrap()),
            Resource::Null => Some(0),
        }
    }

    fn write(&self, buf: &[u8], _offset: usize) -> Option<usize> {
        match self {
            Resource::File(_) => None,
            Resource::Console(stdio) => match *stdio {
                StdIO::Stdin => Some(0),
                StdIO::Stdout => {
                    print!("{}", String::from_utf8_lossy(buf));
                    Some(buf.len())
                }
                StdIO::Stderr => {
                    warn!("{}", String::from_utf8_lossy(buf));
                    Some(buf.len())
                }
            },
            Resource::Random(_) => Some(0),
            Resource::Null => Some(buf.len()),
        }
    }

    fn len(&self) -> usize {
        match self {
            Resource::File(file) => file.length as usize,
            _ => 0,
        }
    }
}
