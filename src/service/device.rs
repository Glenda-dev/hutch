use crate::io::vfio;
use glenda::protocol::device::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DeviceEntry {
    pub desc: DeviceDesc,
    pub allocated: bool,
    pub owner_pid: Option<usize>,
    pub vfio_group: Option<crate::io::vfio::VfioGroup>,
    pub vfio_device: Option<crate::io::vfio::VfioDevice>,
}

pub struct DeviceDatabase {
    pub entries: HashMap<usize, DeviceEntry>,
}

impl DeviceDatabase {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

        pub fn insert_with_vfio(&mut self, id: usize, desc: DeviceDesc, group: Option<crate::io::vfio::VfioGroup>, device: Option<crate::io::vfio::VfioDevice>) {
        self.entries.insert(id, DeviceEntry { desc, allocated: false, owner_pid: None, vfio_group: group, vfio_device: device });
    }
    pub fn get_device(&self, id: usize) -> Option<&crate::io::vfio::VfioDevice> {
        self.entries.get(&id).and_then(|e| e.vfio_device.as_ref())
    }
    
}

pub struct DeviceManager {
    pub db: DeviceDatabase,
    next_id: usize,
    vfio_container: Option<vfio::VfioContainer>,
}

impl DeviceManager {
    pub fn new(config: &crate::config::VfioConfig, kernel: std::sync::Weak<crate::kernel::KernelState>) -> Self {
        let mut db = DeviceDatabase::new();
        let vfio_container = vfio::VfioContainer::new().ok();
        let mut next_id = 1;

        if let Some(container) = &vfio_container {
            for dev_cfg in &config.devices {
                if let Ok(group) = vfio::VfioGroup::new(dev_cfg.group_id) {
                    if group.set_container(container).is_ok() {
                        if let Ok(device) = group.get_device(&dev_cfg.vfio_name) {
                            let mut mmio = vec![];
                            if let Ok((ptr, size)) = device.map_region(0) {
                                mmio.push(MMIORegion { base_addr: ptr, size });
                            }

                            // Setup IRQ thread
                            let mut irq = vec![];
                            let efd = unsafe { libc::eventfd(0, 0) };
                            if efd >= 0 {
                                if device.enable_irq(0, efd).is_ok() {
                                    let irq_num = next_id; // Just use id as irq num for simplicity
                                    irq.push(irq_num);

                                    let kernel = kernel.clone(); std::thread::spawn(move || {
                                        let mut buf = [0u8; 8];
                                        loop {
                                            let n = unsafe {
                                                libc::read(
                                                    efd,
                                                    buf.as_mut_ptr() as *mut libc::c_void,
                                                    8,
                                                )
                                            };
                                            if n == 8 {
                                                if let Some(k) = kernel.upgrade() { k.trigger_irq(irq_num); }
                                            } else if n < 0 {
                                                break;
                                            }
                                        }
                                    });
                                }
                            }

                            db.insert_with_vfio(
                                next_id,
                                DeviceDesc {
                                    name: dev_cfg.name.clone(),
                                    compatible: dev_cfg.compatible.clone(),
                                    mmio,
                                    irq,
                                },
                                Some(group),
                                Some(device),
                            );
                            next_id += 1;
                        }
                    }
                }
            }
        }

        Self { db, next_id, vfio_container }
    }

    pub fn handle_device_call(
        &mut self,
        method: usize,
        tag_label: usize,
        mrs: Vec<usize>,
        utcb: &mut glenda::ipc::utcb::UTCB,
        cspace: &std::sync::RwLock<crate::kernel::cspace::CSpace>,
    ) -> (usize, Vec<usize>) {
        if method == glenda::cap::ipcmethod::CALL || method == glenda::cap::ipcmethod::SEND {
            match tag_label {
                SCAN_PLATFORM => {
                    let mut nodes = Vec::new();
                    for (_id, entry) in &self.db.entries {
                        let desc = &entry.desc;
                        nodes.push(DeviceDescNode {
                            parent: 0,
                            desc: desc.clone(),
                            meta: DeviceNodeMeta::default(),
                        });
                    }
                    let data = bincode::serialize(&nodes).unwrap_or_default();
                    let ptr = utcb.get_buffer_ptr();
                    unsafe {
                        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
                    }
                    utcb.set_buffer_len(data.len());
                    (0, vec![])
                }
                GET_MMIO => {
                    let id = mrs[0];
                    if let Some(entry) = self.db.entries.get(&id) {
                        let desc = &entry.desc;
                        if desc.mmio.len() > 0 {
                            // Pseudo frame cap representation for test
                            let offset = desc.mmio[0].base_addr;
                            let size = desc.mmio[0].size;
                            // Optionally map to vfio if applicable
                            return (0, vec![offset, size]);
                        }
                    }
                    (usize::MAX, vec![])
                }
                GET_IRQ => {
                    let recv = utcb.get_recv_window().bits();
                    let id = mrs[0];
                    if recv != 0 {
                        // find device by id to get irq
                        if let Some(entry) = self.db.entries.get(&id) {
                            let desc = &entry.desc;
                            if desc.irq.len() > 0 {
                                let irq_num = desc.irq[0] as usize;
                                let mut cs = cspace.write().unwrap();
                                cs.insert(
                                    recv,
                                    crate::kernel::capability::Capability::new(
                                        crate::kernel::capability::CapType::IrqHandler,
                                    )
                                    .with_badge(irq_num),
                                );
                            }
                        }
                    }
                    (0, vec![0])
                }
                _ => (0, vec![]),
            }
        } else {
            (0, vec![])
        }
    }
}

impl glenda::interface::device::DeviceService for &mut DeviceManager {
    fn scan_platform(&mut self, _badge: glenda::ipc::Badge) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn get_mmio(&mut self, _badge: glenda::ipc::Badge, _id: usize, _recv: glenda::cap::CapPtr) -> Result<(glenda::cap::Page, usize, usize), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported) // We handle this directly in handle_device_call instead
    }

    fn get_irq(&mut self, _badge: glenda::ipc::Badge, _id: usize, _recv: glenda::cap::CapPtr) -> Result<glenda::cap::IrqHandler, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn report_frame(&mut self, _badge: glenda::ipc::Badge, _frame: glenda::cap::CapPtr, _byte_len: usize) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn report(&mut self, _badge: glenda::ipc::Badge, _desc: std::vec::Vec<glenda::protocol::device::DeviceDescNode>) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn report_state(&mut self, _badge: glenda::ipc::Badge, _status: glenda::protocol::init::ServiceState) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn update(&mut self, _badge: glenda::ipc::Badge, _compatible: std::vec::Vec<std::string::String>) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn register_logic(&mut self, _badge: glenda::ipc::Badge, _desc: glenda::protocol::device::LogicDeviceDesc, _endpoint: glenda::cap::CapPtr) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn alloc_logic(&mut self, _badge: glenda::ipc::Badge, _dev_type: glenda::protocol::device::LogicDeviceType, _criteria: &str, _recv: glenda::cap::CapPtr) -> Result<glenda::cap::Endpoint, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn query(&mut self, _badge: glenda::ipc::Badge, _query: glenda::protocol::device::DeviceQuery) -> Result<std::vec::Vec<std::string::String>, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn get_desc(&mut self, _badge: glenda::ipc::Badge, _name: &str) -> Result<glenda::protocol::device::DeviceDesc, glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn get_logic_desc(&mut self, _badge: glenda::ipc::Badge, _name: &str) -> Result<(usize, glenda::protocol::device::LogicDeviceDesc), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn hook(&mut self, _badge: glenda::ipc::Badge, _target: glenda::protocol::device::HookTarget, _endpoint: glenda::cap::CapPtr) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }

    fn unhook(&mut self, _badge: glenda::ipc::Badge, _target: glenda::protocol::device::HookTarget) -> Result<(), glenda::error::Error> {
        Err(glenda::error::Error::NotSupported)
    }
}
