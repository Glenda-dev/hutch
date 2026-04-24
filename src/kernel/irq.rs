use glenda::error::Error;
use std::sync::RwLock;

pub const MAX_IRQS: usize = 256;

#[derive(Clone)]
pub struct IrqSlot {
    pub cptr: Option<usize>,
    pub badge: usize,
    pub enabled: bool,
}

impl IrqSlot {
    const fn new() -> Self {
        Self { cptr: None, badge: 0, enabled: false }
    }
}

pub struct IrqManager {
    pub table: RwLock<[IrqSlot; MAX_IRQS]>,
}

impl IrqManager {
    pub fn new() -> Self {
        Self { table: RwLock::new([const { IrqSlot::new() }; MAX_IRQS]) }
    }

    pub fn bind_notification(&self, irq: usize, cptr: usize, badge: usize) -> Result<(), Error> {
        println!("[hutch] irq: Binding irq {} to cptr {} badge {}", irq, cptr, badge);
        let mut tbl = self.table.write().unwrap();
        if irq >= MAX_IRQS {
            return Err(Error::InvalidArgs);
        }
        tbl[irq].cptr = Some(cptr);
        tbl[irq].badge = badge;
        tbl[irq].enabled = true;
        Ok(())
    }

    pub fn clear_notification(&self, irq: usize) -> Result<(), Error> {
        println!("[hutch] irq: Clearing irq {}", irq);
        let mut tbl = self.table.write().unwrap();
        if irq >= MAX_IRQS {
            return Err(Error::InvalidArgs);
        }
        tbl[irq].cptr = None;
        tbl[irq].enabled = false;
        Ok(())
    }
}
