
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::FromRawFd;
use std::ffi::CStr;
use std::sync::Mutex;
use std::collections::HashMap;

pub struct VirtualTerminal {
    pub id: usize,
    pub name: String,
    pub master: File,
    pub slave_name: String,
}

pub struct TerminalManager {
    pub vts: Mutex<HashMap<usize, VirtualTerminal>>,
    pub next_vt_id: Mutex<usize>,
}

impl TerminalManager {
    pub fn new() -> Self {
        let mgr = Self {
            vts: Mutex::new(HashMap::new()),
            next_vt_id: Mutex::new(1),
        };
        mgr.create_vt_internal(0, "tty0").unwrap();
        mgr
    }

    pub fn create_vt_internal(&self, id: usize, name: &str) -> std::io::Result<()> {
        unsafe {
            let master_fd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            if master_fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::grantpt(master_fd) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::unlockpt(master_fd) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let pts_name = libc::ptsname(master_fd);
            if pts_name.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let slave_name = CStr::from_ptr(pts_name).to_string_lossy().into_owned();
            
            let vt = VirtualTerminal {
                id,
                name: name.to_string(),
                master: File::from_raw_fd(master_fd),
                slave_name,
            };
            self.vts.lock().unwrap().insert(id, vt);
        }
        Ok(())
    }

    pub fn get_slave_name(&self, id: usize) -> Option<String> {
        self.vts.lock().unwrap().get(&id).map(|vt| vt.slave_name.clone())
    }

    pub fn write(&self, id: usize, buf: &[u8]) -> io::Result<usize> {
        if let Some(mut vt) = self.vts.lock().unwrap().remove(&id) {
            let res = vt.master.write(buf);
            self.vts.lock().unwrap().insert(id, vt);
            res
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "VT not found"))
        }
    }

    pub fn read(&self, id: usize, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(mut vt) = self.vts.lock().unwrap().remove(&id) {
            let res = vt.master.read(buf);
            self.vts.lock().unwrap().insert(id, vt);
            res
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "VT not found"))
        }
    }

    pub fn flush(&self, id: usize) -> io::Result<()> {
        if let Some(mut vt) = self.vts.lock().unwrap().remove(&id) {
            let res = vt.master.flush();
            self.vts.lock().unwrap().insert(id, vt);
            res
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "VT not found"))
        }
    }
}

impl glenda::interface::terminal::VirtualTerminalService for &TerminalManager {
    fn create_vt(&mut self, _badge: glenda::ipc::Badge, name: &str, _recv: glenda::cap::CapPtr) -> Result<(usize, glenda::cap::Endpoint), glenda::error::Error> {
        let mut next_id = self.next_vt_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        self.create_vt_internal(id, name).map_err(|_| glenda::error::Error::IoError)?;
        Ok((id, glenda::cap::Endpoint::from(glenda::cap::CapPtr::null())))
    }

    fn destroy_vt(&mut self, _badge: glenda::ipc::Badge, vt_id: usize) -> Result<(), glenda::error::Error> {
        self.vts.lock().unwrap().remove(&vt_id);
        Ok(())
    }

    fn list_vts(&mut self, _badge: glenda::ipc::Badge) -> Result<std::vec::Vec<glenda::protocol::terminal::VTDesc>, glenda::error::Error> {
        let mut descs = Vec::new();
        for (id, vt) in self.vts.lock().unwrap().iter() {
            descs.push(glenda::protocol::terminal::VTDesc {
                id: *id,
                name: vt.name.clone(),
                mode: glenda::protocol::terminal::TerminalDisplayMode::Text,
                seat_ids: vec![],
            });
        }
        Ok(descs)
    }

    fn list_seats(&mut self, _badge: glenda::ipc::Badge) -> Result<std::vec::Vec<glenda::protocol::terminal::SeatDesc>, glenda::error::Error> {
        Ok(vec![])
    }

    fn switch_vt(&mut self, _badge: glenda::ipc::Badge, _seat_id: usize, _vt_id: usize) -> Result<(), glenda::error::Error> {
        Ok(())
    }

    fn bind_seat(&mut self, _badge: glenda::ipc::Badge, _seat_id: usize, _vt_id: usize) -> Result<(), glenda::error::Error> {
        Ok(())
    }

    fn assign_device_to_seat(&mut self, _badge: glenda::ipc::Badge, _seat_id: usize, _device_name: &str) -> Result<(), glenda::error::Error> {
        Ok(())
    }

    fn revoke_device_from_seat(&mut self, _badge: glenda::ipc::Badge, _seat_id: usize, _device_name: &str) -> Result<(), glenda::error::Error> {
        Ok(())
    }
}
