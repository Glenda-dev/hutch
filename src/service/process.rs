use std::process::{Command, Child};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ProcessManager {
    processes: Mutex<HashMap<usize, Child>>,
    next_pid: Mutex<usize>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
            next_pid: Mutex::new(100),
        }
    }

    pub fn spawn(&self, path: &str, args: Vec<String>) -> Result<usize, std::io::Error> {
        let child = Command::new(path)
            .args(args)
            .spawn()?;
        
        let mut next_pid = self.next_pid.lock().unwrap();
        let pid = *next_pid;
        *next_pid += 1;
        
        self.processes.lock().unwrap().insert(pid, child);
        Ok(pid)
    }

    pub fn kill(&self, pid: usize) -> bool {
        if let Some(mut child) = self.processes.lock().unwrap().remove(&pid) {
            let _ = child.kill();
            true
        } else {
            false
        }
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
