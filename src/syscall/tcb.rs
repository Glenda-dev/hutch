use crate::kernel::KernelState;

impl KernelState {
    pub fn handle_tcb_invocation(
        &self,
        _method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        // TCB configure, etc. In hosted mode, we might just track state
        (0, vec![])
    }
}
