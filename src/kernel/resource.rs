use crate::kernel::KernelState;
use glenda::cap::cnodemethod;

impl KernelState {
    pub fn handle_cnode_invocation(
        &self,
        method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        match method {
            cnodemethod::MINT | cnodemethod::COPY => {
                // In hosted, we just track this in the local CSpace map for simulation
                (0, vec![])
            }
            cnodemethod::DELETE | cnodemethod::REVOKE => (0, vec![]),
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
