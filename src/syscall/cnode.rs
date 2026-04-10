use crate::kernel::KernelState;
use glenda::cap::cnodemethod;

impl KernelState {
    pub fn handle_cnode_invocation(
        &self,
        method: usize,
        _tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        match method {
            cnodemethod::MINT => {
                let src_cptr = mrs[0];
                let dest_cptr = mrs[1];
                let badge = mrs[2];
                let mut cspace = self.cspace.write().unwrap();
                if let Some(cap) = cspace.get(src_cptr).cloned() {
                    cspace.insert_derived(src_cptr, dest_cptr, cap.with_badge(badge));
                    return (0, vec![]);
                }
                (glenda::error::Error::InvalidCapability as usize, vec![])
            }
            cnodemethod::COPY => {
                let src_cptr = mrs[0];
                let dest_cptr = mrs[1];
                let mut cspace = self.cspace.write().unwrap();
                if let Some(cap) = cspace.get(src_cptr).cloned() {
                    cspace.insert_derived(src_cptr, dest_cptr, cap);
                    return (0, vec![]);
                }
                (glenda::error::Error::InvalidCapability as usize, vec![])
            }
            cnodemethod::DELETE => {
                let cptr = mrs[0];
                let mut cspace = self.cspace.write().unwrap();
                cspace.delete(cptr);
                (0, vec![])
            }
            cnodemethod::REVOKE => {
                let cptr = mrs[0];
                let mut cspace = self.cspace.write().unwrap();
                cspace.revoke(cptr);
                (0, vec![])
            }
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
