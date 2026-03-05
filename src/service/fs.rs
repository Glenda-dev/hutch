use std::path::{PathBuf};
use std::fs::File;
use std::io::{self};
use std::os::unix::fs::FileExt;

pub struct Sandbox {
    root: String,
}

impl Sandbox {
    pub fn new(root: &str) -> Self {
        Self { 
            root: root.to_string(),
        }
    }

    pub fn map_path(&self, glenda_path: &str) -> PathBuf {
        let mut path = PathBuf::from(&self.root);
        path.push(glenda_path.trim_start_matches('/'));
        path
    }

    pub fn read_at(&self, fd: &File, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        fd.read_at(buf, offset)
    }

    pub fn write_at(&self, fd: &File, offset: u64, buf: &[u8]) -> io::Result<usize> {
        fd.write_at(buf, offset)
    }
}
