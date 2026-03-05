use crate::kernel::KernelState;
use glenda::cap::tcpmethod;

impl KernelState {
    pub fn handle_tcb_invocation(
        &self,
        _method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        // TCB configure, status, etc.
        (0, vec![])
    }
}
