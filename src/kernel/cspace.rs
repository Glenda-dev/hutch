use crate::kernel::capability::Capability;
use std::collections::HashMap;

pub struct CSpace {
    pub slots: HashMap<usize, Capability>,
    pub cdt: HashMap<usize, Vec<usize>>, // Capability Derivation Tree: parent -> children
}

impl CSpace {
    pub fn new() -> Self {
        Self { slots: HashMap::new(), cdt: HashMap::new() }
    }

    pub fn insert(&mut self, cptr: usize, cap: Capability) {
        self.slots.insert(cptr, cap);
    }

    pub fn insert_derived(&mut self, parent: usize, child: usize, cap: Capability) {
        self.slots.insert(child, cap);
        self.cdt.entry(parent).or_insert_with(Vec::new).push(child);
    }

    pub fn delete(&mut self, cptr: usize) {
        self.slots.remove(&cptr);

        // Remove from parent's CDT tracking
        for children in self.cdt.values_mut() {
            if let Some(pos) = children.iter().position(|x| *x == cptr) {
                children.remove(pos);
            }
        }
    }

    pub fn revoke(&mut self, parent: usize) {
        if let Some(children) = self.cdt.remove(&parent) {
            for child in children {
                self.revoke(child);
                self.slots.remove(&child);
            }
        }
    }

    pub fn get(&self, cptr: usize) -> Option<&Capability> {
        self.slots.get(&cptr)
    }
}
