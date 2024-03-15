use super::*;
use alloc::string::ToString;
use core::ops::Deref;

pub const PATH_SEPARATOR: char = '/';
pub const PATH_SEPARATOR_STR: &str = "/";

#[derive(Clone, Debug)]
pub struct FsPath {
    path: Arc<str>,
    fs: Arc<Box<dyn FileSystem>>,
}

impl FsPath {
    /// Creates a root path for the given filesystem
    pub fn root(fs: Arc<Box<dyn FileSystem>>) -> Self {
        Self {
            path: Arc::from(""),
            fs,
        }
    }

    /// Appends a path segment to this path, returning the result
    pub fn join(&self, path: impl AsRef<str>) -> Result<Self> {
        let new_path = join_internal(&self.path, path.as_ref())?;
        Ok(Self {
            path: Arc::from(new_path),
            fs: self.fs.clone(),
        })
    }

    /// Returns the root path of this filesystem
    pub fn root_path(&self) -> Self {
        Self {
            path: Arc::from(""),
            fs: self.fs.clone(),
        }
    }

    /// Returns true if this is the root path
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    /// Iterates over all entries of this directory path
    pub fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Self> + Send>> {
        let parent = self.path.clone();
        let fs = self.fs.clone();
        Ok(Box::new(self.fs.read_dir(&self.path)?.map(move |path| {
            Self {
                path: format!("{}/{}", parent, path.name).into(),
                fs: fs.clone(),
            }
        })))
    }

    /// Opens the file at this path for reading
    pub fn open_file(&self) -> Result<FileHandle> {
        self.fs.open_file(&self.path)
    }

    /// Returns the entry metadata for the file at this path
    pub fn metadata(&self) -> Result<Metadata> {
        self.fs.metadata(&self.path)
    }

    /// Returns true if a file or directory exists at this path, false otherwise
    pub fn exists(&self) -> Result<bool> {
        self.fs.exists(&self.path)
    }

    /// Returns `true` if the path exists and is a regular file
    pub fn is_file(&self) -> Result<bool> {
        if !self.exists()? {
            return Ok(false);
        }
        self.metadata().map(|m| m.is_file())
    }

    /// Returns `true` if the path exists and is a directory
    pub fn is_dir(&self) -> Result<bool> {
        if !self.exists()? {
            return Ok(false);
        }
        self.metadata().map(|m| m.is_dir())
    }

    /// Returns the extension of this path, if any
    pub fn extension(&self) -> Option<String> {
        extension_internal(&self.path)
    }

    /// Returns the filename of this path
    pub fn filename(&self) -> String {
        filename_internal(&self.path)
    }

    /// Returns the parent directory of this path
    pub fn parent(&self) -> Self {
        Self {
            path: Arc::from(parent_internal(&self.path)),
            fs: self.fs.clone(),
        }
    }
}

impl Deref for FsPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl PartialEq for FsPath {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && Arc::ptr_eq(&self.fs, &other.fs)
    }
}

impl Eq for FsPath {}

fn extension_internal(path: &str) -> Option<String> {
    let filename = filename_internal(path);
    let mut parts = filename.rsplitn(2, '.');
    let after = parts.next();
    let before = parts.next();
    match before {
        None | Some("") => None,
        _ => after.map(|x| x.to_string()),
    }
}

fn filename_internal(path: &str) -> String {
    let index = path.rfind('/').map(|x| x + 1).unwrap_or(0);
    path[index..].to_string()
}

fn parent_internal(path: &str) -> String {
    let index = path.rfind(PATH_SEPARATOR);
    index
        .map(|idx| path[..idx].to_string())
        .unwrap_or_else(|| "".to_string())
}

fn join_internal(in_path: &str, path: &str) -> Result<String> {
    if path.is_empty() {
        return Ok(in_path.to_string());
    }

    // Prevent paths from ending in slashes unless this is just the root directory.
    if path.len() > 1 && path.ends_with(PATH_SEPARATOR) {
        return Err(FsError::InvalidPath(path.into()));
    }

    let mut new_components: Vec<&str> = vec![];
    let mut base_path = if path.starts_with(PATH_SEPARATOR) {
        "".to_string()
    } else {
        in_path.to_string()
    };

    for component in path.split(PATH_SEPARATOR) {
        match component {
            "." | "" => continue,
            ".." => {
                if !new_components.is_empty() {
                    new_components.truncate(new_components.len() - 1);
                } else {
                    base_path = parent_internal(&base_path);
                }
            }
            _ => new_components.push(component),
        }
    }

    Ok(new_components.join(PATH_SEPARATOR_STR))
}
