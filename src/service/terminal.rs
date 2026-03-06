use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::FromRawFd;

pub struct TerminalManager {
    stdin: File,
    stdout: File,
}

impl TerminalManager {
    pub fn new() -> Self {
        unsafe {
            Self {
                stdin: File::from_raw_fd(0),
                stdout: File::from_raw_fd(1),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let mut out = &self.stdout;
        out.write(buf)
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut input = &self.stdin;
        input.read(buf)
    }

    pub fn flush(&self) -> io::Result<()> {
        let mut out = &self.stdout;
        out.flush()
    }
}
