use crate::kernel::KernelState;
use glenda::cap::{cnodemethod, CapType};

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
            cnodemethod::MOVE => {
                let src_cptr = mrs[0];
                let dest_cptr = mrs[1];
                let mut cspace = self.cspace.write().unwrap();
                if let Some(cap) = cspace.get(src_cptr).cloned() {
                    cspace.insert(dest_cptr, cap);
                    cspace.delete(src_cptr);
                    
                    // Transfer children in CDT logic (simplified by just moving the raw cap)
                    if let Some(children) = cspace.cdt.remove(&src_cptr) {
                        cspace.cdt.insert(dest_cptr, children);
                    }

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
            cnodemethod::RECYCLE => {
                let cptr = mrs[0];
                let mut cspace = self.cspace.write().unwrap();
                cspace.revoke(cptr);
                
                let is_untyped;
                if let Some(cap) = cspace.get(cptr) {
                    is_untyped = cap.cap_type == CapType::Untyped;
                } else {
                    is_untyped = false;
                }

                if is_untyped {
                    // Try to reset cap data to purely untyped representing free memory block
                    if let Some(cap) = cspace.get(cptr) {
                        cap.set_data(crate::kernel::capability::CapData::None);
                    }
                    (0, vec![0, 0])
                } else {
                    cspace.delete(cptr);
                    (0, vec![0, 0])
                }
            }
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
