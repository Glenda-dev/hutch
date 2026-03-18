use crate::kernel::KernelState;
use glenda::cap::vspacemethod;

impl KernelState {
    pub fn handle_vspace_invocation(
        &self,
        method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        match method {
            vspacemethod::MAP | vspacemethod::MAP_TABLE | vspacemethod::SETUP => {
                // Return success as address space is host-managed
                (0, vec![])
            }
            _ => (0, vec![]),
        }
    }
}
