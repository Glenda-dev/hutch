use crate::kernel::endpoint::Endpoint;
use crate::service::uring::UringEmulator;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ResourceManager {
    pub heap_start: usize,
    pub heap_size: usize,
    pub brk: usize,
    pub registered_caps: HashMap<usize, usize>, // endpoint_id -> cptr
    pub uring_emulators: HashMap<usize, UringEmulator>, // dummy_id -> emulator
    pub endpoints: HashMap<usize, Arc<Endpoint>>, // cptr -> endpoint
}

impl ResourceManager {
    pub fn new(heap_start: usize, heap_size: usize) -> Self {
        Self {
            heap_start,
            heap_size,
            brk: heap_start,
            registered_caps: HashMap::new(),
            uring_emulators: HashMap::new(),
            endpoints: HashMap::new(),
        }
    }

    pub fn sbrk(&mut self, incr: isize) -> usize {
        let old_brk = self.brk;
        self.brk = (self.brk as isize + incr) as usize;
        old_brk
    }

    pub fn register_cap(&mut self, endpoint_id: usize, cptr: usize) {
        self.registered_caps.insert(endpoint_id, cptr);
    }

    pub fn get_registered_cap(&self, endpoint_id: usize) -> Option<usize> {
        self.registered_caps.get(&endpoint_id).cloned()
    }

    pub fn register_uring(&mut self, id: usize, emulator: UringEmulator) {
        self.uring_emulators.insert(id, emulator);
    }

    pub fn process_uring(&mut self, id: usize) -> std::io::Result<()> {
        if let Some(emu) = self.uring_emulators.get_mut(&id) {
            emu.process_requests()?;
        }
        Ok(())
    }

    pub fn get_endpoint(&mut self, cptr: usize) -> Arc<Endpoint> {
        self.endpoints.entry(cptr).or_insert_with(|| Arc::new(Endpoint::new())).clone()
    }
}
