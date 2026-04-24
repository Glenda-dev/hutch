use crate::io::uring::UringEmulator;
use crate::kernel::endpoint::Endpoint;
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

impl glenda::interface::resource::ResourceService for ResourceManager {
    fn alloc(
        &mut self,
        _pid: glenda::ipc::Badge,
        _obj_type: glenda::cap::CapType,
        _flags: usize,
        _recv: glenda::cap::CapPtr,
    ) -> Result<glenda::cap::CapPtr, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn dma_alloc(
        &mut self,
        _pid: glenda::ipc::Badge,
        _pages: usize,
        _recv: glenda::cap::CapPtr,
    ) -> Result<(usize, glenda::cap::Page), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn free(
        &mut self,
        _pid: glenda::ipc::Badge,
        _cap: glenda::cap::CapPtr,
    ) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn get_cap(
        &mut self,
        _pid: glenda::ipc::Badge,
        _cap_type: glenda::protocol::resource::ResourceType,
        _id: usize,
        _recv: glenda::cap::CapPtr,
    ) -> Result<glenda::cap::CapPtr, glenda::error::Error> {
        if _cap_type == glenda::protocol::resource::ResourceType::Endpoint {
            if let Some(cptr) = self.get_registered_cap(_id) {
                return Ok(glenda::cap::CapPtr::from(cptr));
            }
            return Err(glenda::error::Error::NotFound);
        }
        Err(glenda::error::Error::NotSupported)
    }

    fn register_cap(
        &mut self,
        _pid: glenda::ipc::Badge,
        _cap_type: glenda::protocol::resource::ResourceType,
        _id: usize,
        _cap: glenda::cap::CapPtr,
    ) -> Result<(), glenda::error::Error> {
        if _cap_type == glenda::protocol::resource::ResourceType::Endpoint {
            ResourceManager::register_cap(self, _id, _cap.bits());
            return Ok(());
        }
        Err(glenda::error::Error::NotSupported)
    }

    fn get_config(
        &mut self,
        _pid: glenda::ipc::Badge,
        _name: &str,
        _recv: glenda::cap::CapPtr,
    ) -> Result<(glenda::cap::Page, usize), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn status(
        &mut self,
        _pid: glenda::ipc::Badge,
    ) -> Result<glenda::protocol::resource::WarrenStatus, glenda::error::Error> {
        let used = self.brk.saturating_sub(self.heap_start);
        let available = self.heap_size.saturating_sub(used);
        Ok(glenda::protocol::resource::WarrenStatus {
            memory: glenda::protocol::resource::MemoryStatus {
                available_bytes: available,
                total_bytes: self.heap_size,
            },
        })
    }
}
