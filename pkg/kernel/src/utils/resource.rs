use alloc::string::String;
use pc_keyboard::DecodedKey;

use crate::input::try_get_key;

#[derive(Debug, Clone)]
pub enum StdIO {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub enum Resource {
    Console(StdIO),
    Null,
}

impl Resource {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, ()> {
        match self {
            Resource::Console(stdio) => match stdio {
                &StdIO::Stdin => {
                    return if buf.len() < 4 {
                        Ok(0)
                    } else {
                        // TODO: get key async
                        if let Some(DecodedKey::Unicode(k)) = try_get_key() {
                            let s = k.encode_utf8(buf);
                            Ok(s.len())
                        } else {
                            Ok(0)
                        }
                    };
                }
                _ => Err(()),
            },
            Resource::Null => Ok(0),
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, ()> {
        match self {
            Resource::Console(stdio) => match *stdio {
                StdIO::Stdin => Err(()),
                StdIO::Stdout => {
                    print!("{}", String::from_utf8_lossy(buf));
                    Ok(buf.len())
                }
                StdIO::Stderr => {
                    warn!("{}", String::from_utf8_lossy(buf));
                    Ok(buf.len())
                }
            },
            Resource::Null => Ok(0),
        }
    }
}
