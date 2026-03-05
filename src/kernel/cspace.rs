use std::collections::HashMap;
use crate::kernel::capability::Capability;

pub struct CSpace {
    pub slots: HashMap<usize, Capability>,
}

impl CSpace {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn insert(&mut self, cptr: usize, cap: Capability) {
        self.slots.insert(cptr, cap);
    }

    pub fn get(&self, cptr: usize) -> Option<&Capability> {
        self.slots.get(&cptr)
    }
}
