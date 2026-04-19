use glenda::error::Error;
use glenda::interface::fs::{FileHandleService, FileSystemService};
use glenda::protocol::fs::seek;
use glenda::protocol::fs::{OpenFlags, Stat};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Sandbox {
    root: String,
    files: Mutex<HashMap<usize, File>>,
    next_fd: Mutex<usize>,
}

impl Sandbox {
    pub fn new(root: &str) -> Self {
        Self { root: root.to_string(), files: Mutex::new(HashMap::new()), next_fd: Mutex::new(10) }
    }

    pub fn map_path(&self, glenda_path: &str) -> PathBuf {
        let mut path = PathBuf::from(&self.root);
        path.push(glenda_path.trim_start_matches('/'));
        path
    }
}

impl FileSystemService for &Sandbox {
    fn open(
        &mut self,
        _pid: glenda::ipc::Badge,
        path: &str,
        flags: OpenFlags,
        _mode: u32,
        _recv_slot: glenda::cap::CapPtr,
    ) -> Result<usize, Error> {
        let real_path = self.map_path(path);
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

        match opts.open(real_path) {
            Ok(f) => {
                let mut fd_gen = self.next_fd.lock().unwrap();
                let fd = *fd_gen;
                *fd_gen += 1;
                self.files.lock().unwrap().insert(fd, f);
                Ok(fd)
            }
            Err(_) => Err(Error::IoError),
        }
    }

    fn mkdir(&mut self, _pid: glenda::ipc::Badge, path: &str, _mode: u32) -> Result<(), Error> {
        let rp = self.map_path(path);
        std::fs::create_dir(rp).map_err(|_| Error::IoError)
    }

    fn unlink(&mut self, _pid: glenda::ipc::Badge, path: &str) -> Result<(), Error> {
        let rp = self.map_path(path);
        std::fs::remove_file(rp).map_err(|_| Error::IoError)
    }

    fn rename(
        &mut self,
        _pid: glenda::ipc::Badge,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Error> {
        let o = self.map_path(old_path);
        let n = self.map_path(new_path);
        std::fs::rename(o, n).map_err(|_| Error::IoError)
    }

    fn link(
        &mut self,
        _pid: glenda::ipc::Badge,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Error> {
        let old_real = self.map_path(old_path);
        let new_real = self.map_path(new_path);
        std::fs::hard_link(old_real, new_real).map_err(|_| Error::IoError)
    }

    fn stat_path(&mut self, _pid: glenda::ipc::Badge, path: &str) -> Result<Stat, Error> {
        let rp = self.map_path(path);
        let meta = std::fs::metadata(rp).map_err(|_| Error::IoError)?;

        let mut st = Stat::default();
        st.dev = meta.dev() as usize;
        st.ino = meta.ino() as usize;
        st.mode = meta.mode();
        st.nlink = meta.nlink() as u32;
        st.uid = meta.uid();
        st.gid = meta.gid();
        st.rdev = meta.rdev() as usize;
        st.size = meta.size() as usize;
        st.blksize = meta.blksize() as u32;
        st.blocks = meta.blocks() as usize;
        st.atime = meta.atime() as usize;
        st.mtime = meta.mtime() as usize;
        st.ctime = meta.ctime() as usize;

        Ok(st)
    }
}

impl FileHandleService for &Sandbox {
    fn close(&mut self, pid: glenda::ipc::Badge) -> Result<(), Error> {
        if self.files.lock().unwrap().remove(&(pid.bits())).is_some() {
            Ok(())
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn stat(&self, pid: glenda::ipc::Badge) -> Result<Stat, Error> {
        let files = self.files.lock().unwrap();
        let f = files.get(&(pid.bits())).ok_or(Error::InvalidArgs)?;
        let meta = f.metadata().map_err(|_| Error::IoError)?;

        let mut st = Stat::default();
        st.dev = meta.dev() as usize;
        st.ino = meta.ino() as usize;
        st.mode = meta.mode();
        st.nlink = meta.nlink() as u32;
        st.uid = meta.uid();
        st.gid = meta.gid();
        st.rdev = meta.rdev() as usize;
        st.size = meta.size() as usize;
        st.blksize = meta.blksize() as u32;
        st.blocks = meta.blocks() as usize;
        st.atime = meta.atime() as usize;
        st.mtime = meta.mtime() as usize;
        st.ctime = meta.ctime() as usize;

        Ok(st)
    }

    fn read(
        &mut self,
        pid: glenda::ipc::Badge,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, Error> {
        let files = self.files.lock().unwrap();
        if let Some(f) = files.get(&(pid.bits())) {
            f.read_at(buf, offset as u64).map_err(|_| Error::IoError)
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn write(
        &mut self,
        pid: glenda::ipc::Badge,
        offset: usize,
        buf: &[u8],
    ) -> Result<usize, Error> {
        let files = self.files.lock().unwrap();
        if let Some(f) = files.get(&(pid.bits())) {
            f.write_at(buf, offset as u64).map_err(|_| Error::IoError)
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn getdents(
        &mut self,
        pid: glenda::ipc::Badge,
        count: usize,
    ) -> Result<Vec<glenda::protocol::fs::DEntry>, Error> {
        let mut files = self.files.lock().unwrap();
        let f = files.get_mut(&(pid.bits())).ok_or(Error::InvalidArgs)?;

        let fd = f.as_raw_fd();
        let mut buf = vec![0u8; count.max(4096)];
        let res = unsafe { libc::syscall(libc::SYS_getdents64, fd, buf.as_mut_ptr(), buf.len()) };

        if res < 0 {
            return Err(Error::IoError);
        }

        let mut dentries = Vec::new();
        let mut offset = 0;
        let res = res as usize;

        while offset < res {
            let dirent = unsafe { &*(buf.as_ptr().add(offset) as *const libc::dirent64) };

            let mut name = [0u8; 256];
            let name_len = unsafe { libc::strlen(dirent.d_name.as_ptr()) } as usize;
            let copy_len = name_len.min(255);

            for i in 0..copy_len {
                name[i] = dirent.d_name[i] as u8;
            }

            dentries.push(glenda::protocol::fs::DEntry {
                d_ino: dirent.d_ino as usize,
                d_off: dirent.d_off as i64,
                d_reclen: dirent.d_reclen,
                d_type: dirent.d_type,
                d_name: name,
            });

            offset += dirent.d_reclen as usize;
        }

        Ok(dentries)
    }

    fn seek(
        &mut self,
        pid: glenda::ipc::Badge,
        offset: i64,
        whence: usize,
    ) -> Result<usize, Error> {
        let mut files = self.files.lock().unwrap();
        let f = files.get_mut(&(pid.bits())).ok_or(Error::InvalidArgs)?;

        let pos = match whence {
            seek::SEEK_SET => std::io::SeekFrom::Start(offset as u64),
            seek::SEEK_CUR => std::io::SeekFrom::Current(offset),
            seek::SEEK_END => std::io::SeekFrom::End(offset),
            _ => return Err(Error::InvalidArgs),
        };

        use std::io::Seek;
        f.seek(pos).map(|p| p as usize).map_err(|_| Error::IoError)
    }

    fn sync(&mut self, pid: glenda::ipc::Badge) -> Result<(), Error> {
        let files = self.files.lock().unwrap();
        if let Some(f) = files.get(&(pid.bits())) {
            f.sync_all().map_err(|_| Error::IoError)
        } else {
            Err(Error::InvalidArgs)
        }
    }

    fn truncate(&mut self, pid: glenda::ipc::Badge, size: usize) -> Result<(), Error> {
        let files = self.files.lock().unwrap();
        if let Some(f) = files.get(&(pid.bits())) {
            f.set_len(size as u64).map_err(|_| Error::IoError)
        } else {
            Err(Error::InvalidArgs)
        }
    }
}
