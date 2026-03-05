use std::fs::File;
use std::io::{self};
use std::os::unix::fs::FileExt;

pub struct Terminal {
    stdin: File,
    stdout: File,
}

impl Terminal {
    pub fn new() -> Self {
        use std::os::unix::io::FromRawFd;
        unsafe {
            Self {
                stdin: File::from_raw_fd(0),
                stdout: File::from_raw_fd(1),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.stdout.write_at(buf, 0)
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdin.read_at(buf, 0)
    }
}
