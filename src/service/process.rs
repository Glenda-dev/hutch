use std::collections::HashMap;
use std::process::{Child, Command};
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

pub struct ProcessManager {
    processes: Mutex<HashMap<usize, Child>>,
    next_pid: Mutex<usize>,
    terminal: std::sync::Arc<crate::service::terminal::TerminalManager>,
}

impl ProcessManager {
    pub fn new(terminal: std::sync::Arc<crate::service::terminal::TerminalManager>) -> Self {
        Self { processes: Mutex::new(HashMap::new()), next_pid: Mutex::new(100), terminal }
    }

    pub fn wait(&self, pid: usize) -> Option<i32> {
        if let Some(child) = self.processes.lock().unwrap().get_mut(&pid) {
            match child.wait() {
                Ok(status) => status.code(),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

impl glenda::interface::process::ProcessService for &ProcessManager {
    fn spawn(&mut self, _pid: glenda::ipc::Badge, path: &str) -> Result<usize, glenda::error::Error> {
        let child = {
            let slave = std::fs::OpenOptions::new().read(true).write(true).open(self.terminal.get_slave_name(0).unwrap()).unwrap();
            let slave_fd = std::os::unix::io::AsRawFd::as_raw_fd(&slave);
            unsafe {
                Command::new(path)
                    .stdin(std::process::Stdio::from_raw_fd(slave_fd))
                    .stdout(std::process::Stdio::from_raw_fd(slave_fd))
                    .stderr(std::process::Stdio::from_raw_fd(slave_fd))
                    .spawn()
            }
        }.map_err(|_| glenda::error::Error::IoError)?;
        
        let mut next_pid = self.next_pid.lock().unwrap();
        let pid = *next_pid;
        *next_pid += 1;

        self.processes.lock().unwrap().insert(pid, child);
        Ok(pid)
    }

    fn create(&mut self, _pid: glenda::ipc::Badge, _name: &str) -> Result<usize, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn exit(&mut self, _pid: glenda::ipc::Badge, code: usize) -> Result<(), glenda::error::Error> {
        std::process::exit(code as i32);
    }

    fn kill(&mut self, _pid: glenda::ipc::Badge, target: usize) -> Result<(), glenda::error::Error> {
        if let Some(mut child) = self.processes.lock().unwrap().remove(&target) {
            let _ = child.kill();
            Ok(())
        } else {
            Err(glenda::error::Error::InvalidArgs)
        }
    }

    fn get_cnode(&mut self, _pid: glenda::ipc::Badge, _target: usize, _recv: glenda::cap::CapPtr) -> Result<glenda::cap::CNode, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }
}
