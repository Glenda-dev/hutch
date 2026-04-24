use crate::kernel::KernelState;
use crate::kernel::capability::CapType;

impl KernelState {
    pub fn handle_irq_invocation(
        &self,
        irq: usize,
        method: usize,
        _tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::cap::irqmethod;
        match method {
            irqmethod::SET_NOTIFICATION => {
                let ep_cptr = if !mrs.is_empty() { mrs[0] } else { 0 };
                let badge = if mrs.len() > 1 { mrs[1] } else { 0 };
                let cap = self.cspace.read().unwrap().get(ep_cptr).cloned();
                if let Some(cap) = cap {
                    if cap.cap_type == CapType::Endpoint {
                        if let Err(e) = self.irq_manager.bind_notification(irq, ep_cptr, badge) {
                            return (e as usize, vec![]);
                        }
                        return (0, vec![]);
                    }
                }
                (glenda::error::Error::InvalidCapability as usize, vec![])
            }
            irqmethod::ACK => {
                let dev_mgr = self.device.lock().unwrap();
                for (_, entry) in dev_mgr.db.entries.iter() {
                    if entry.desc.irq.contains(&(irq as usize)) {
                        if let Some(dev) = &entry.vfio_device {
                            let _ = dev.unmask_irq(0);
                        }
                    }
                }
                (0, vec![])
            }
            irqmethod::CLEAR_NOTIFICATION => {
                let _ = self.irq_manager.clear_notification(irq);
                (0, vec![])
            }
            irqmethod::SET_PRIORITY => (0, vec![]),
            irqmethod::SET_THRESHOLD => (0, vec![]),
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
