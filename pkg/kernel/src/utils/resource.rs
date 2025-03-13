use alloc::{collections::BTreeMap, string::String, vec::Vec};
use pc_keyboard::DecodedKey;
use spin::Mutex;
use storage::{Device, FileHandle, random::Random};

use crate::input::try_get_key;

#[derive(Debug, Clone)]
pub enum StdIO {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub struct ResourceSet {
    pub handles: BTreeMap<u8, Mutex<Resource>>,
    recycled: Vec<u8>,
}

impl Default for ResourceSet {
    fn default() -> Self {
        let mut res = Self {
            handles: BTreeMap::new(),
            recycled: Vec::new(),
        };

        res.open(Resource::Console(StdIO::Stdin));
        res.open(Resource::Console(StdIO::Stdout));
        res.open(Resource::Console(StdIO::Stderr));

        res
    }
}

impl ResourceSet {
    pub fn open(&mut self, res: Resource) -> u8 {
        let fd = match self.recycled.pop() {
            Some(fd) => fd,
            None => self.handles.len() as u8,
        };
        self.handles.insert(fd, Mutex::new(res));
        fd
    }

    pub fn close(&mut self, fd: u8) -> bool {
        match self.handles.remove(&fd) {
            Some(_) => {
                self.recycled.push(fd);
                true
            }
            None => false,
        }
    }

    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        match self.handles.get(&fd).and_then(|h| h.lock().read(buf)) {
            Some(count) => count as isize,
            None => -1,
        }
    }

    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        match self.handles.get(&fd).and_then(|h| h.lock().write(buf)) {
            Some(count) => count as isize,
            None => -1,
        }
    }
}

pub enum Resource {
    File(FileHandle),
    Console(StdIO),
    Random(Random),
    Null,
}

impl Resource {
    fn read(&mut self, buf: &mut [u8]) -> Option<usize> {
        match self {
            Resource::File(file) => {
                let ret = file.read(buf);
                if let Err(e) = ret {
                    error!("Failed to read file: {:?}", e);
                    None
                } else {
                    Some(ret.unwrap())
                }
            }
            Resource::Console(stdio) => match stdio {
                StdIO::Stdin => Some(if buf.len() < 4 {
                    0
                } else if let Some(DecodedKey::Unicode(k)) = try_get_key() {
                    let s = k.encode_utf8(buf);
                    s.len()
                } else {
                    0
                }),
                _ => Some(0),
            },
            Resource::Random(random) => Some(random.read(buf, 0, buf.len()).unwrap()),
            Resource::Null => Some(0),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Option<usize> {
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
}

impl core::fmt::Debug for Resource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Resource::File(h) => write!(f, "File({})", h.meta.name),
            Resource::Console(c) => write!(f, "Console({:?})", c),
            Resource::Random(_) => write!(f, "Random"),
            Resource::Null => write!(f, "Null"),
        }
    }
}
