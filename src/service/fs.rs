use std::path::{PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{self};
use std::os::unix::fs::FileExt;
use glenda::protocol::fs::OpenFlags;

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

    pub fn open(&self, glenda_path: &str, flags: OpenFlags) -> io::Result<File> {
        let path = self.map_path(glenda_path);
        let mut opts = OpenOptions::new();
        
        if flags.contains(OpenFlags::O_RDWR) {
            opts.read(true).write(true);
        } else if flags.contains(OpenFlags::O_WRONLY) {
            opts.write(true);
        } else {
            opts.read(true);
        }

        if flags.contains(OpenFlags::O_CREAT) {
            opts.create(true);
        }
        if flags.contains(OpenFlags::O_TRUNC) {
            opts.truncate(true);
        }
        if flags.contains(OpenFlags::O_APPEND) {
            opts.append(true);
        }

        opts.open(path)
    }

    pub fn read_at(&self, fd: &File, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        fd.read_at(buf, offset)
    }

    pub fn write_at(&self, fd: &File, offset: u64, buf: &[u8]) -> io::Result<usize> {
        fd.write_at(buf, offset)
    }
}
