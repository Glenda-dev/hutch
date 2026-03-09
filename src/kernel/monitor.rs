use crate::kernel::KernelState;
use glenda::cap::monitormethod;

impl KernelState {
    pub fn handle_monitor_invocation(
        &self,
        method: usize,
        _tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        match method {
            monitormethod::SBRK => {
                let incr = mrs[0] as isize;
                let mut res = self.resource.lock().unwrap();
                let old_brk = res.sbrk(incr);
                (old_brk, vec![])
            }
            monitormethod::EXIT => {
                let code = mrs[0];
                println!("[hutch] Process exited with code: {}", code);
                std::process::exit(code as i32);
            }
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
