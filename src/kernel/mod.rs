pub mod capability;
pub mod cspace;
pub mod endpoint;
pub mod init;
pub mod irq;
pub mod trap;

use crate::config::Config;
use crate::kernel::capability::{CapType, Capability};
use crate::kernel::cspace::CSpace;
use crate::kernel::irq::IrqManager;
use crate::service::fs::Sandbox;
use crate::service::network::NetworkManager;
use crate::service::process::ProcessManager;
use crate::service::resource::ResourceManager;
use crate::service::terminal::TerminalManager;
use crate::service::time::TimeManager;
use std::sync::Arc;
use std::sync::Mutex;

// Well-known CPTRs (synced with libglenda-rs/src/cap/mod.rs)
pub const CSPACE_CPTR: usize = 1;
pub const VSPACE_CPTR: usize = 2;
pub const TCB_CPTR: usize = 3;
pub const MONITOR_CPTR: usize = 4;
pub const CONSOLE_CPTR: usize = 5;
pub const REPLY_CPTR: usize = 6;
pub const ENDPOINT_CPTR: usize = 8;

pub struct KernelState {
    pub irq_manager: Arc<IrqManager>,
    pub cspace: std::sync::RwLock<CSpace>,
    pub resource: Mutex<ResourceManager>,
    pub sandbox: Arc<Sandbox>,
    pub processes: Arc<ProcessManager>,
    pub network: Arc<NetworkManager>,
    pub time: Arc<TimeManager>,
    pub terminal: Arc<TerminalManager>,
    pub device: Arc<Mutex<crate::service::device::DeviceManager>>,
}

impl KernelState {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new_cyclic(|me| {
            let mut cspace = CSpace::new();
            // Setup well-known caps
            cspace.insert(CSPACE_CPTR, Capability::new(CapType::CNode));
            cspace.insert(VSPACE_CPTR, Capability::new(CapType::VSpace));
            cspace.insert(TCB_CPTR, Capability::new(CapType::TCB));
            cspace.insert(MONITOR_CPTR, Capability::new(CapType::Endpoint));
            cspace.insert(CONSOLE_CPTR, Capability::new(CapType::Console));
            cspace.insert(REPLY_CPTR, Capability::new(CapType::Reply));
            cspace.insert(ENDPOINT_CPTR, Capability::new(CapType::Endpoint));

            let terminal = Arc::new(TerminalManager::new());
            Self {
                cspace: std::sync::RwLock::new(cspace),
                irq_manager: Arc::new(IrqManager::new()),
                resource: Mutex::new(ResourceManager::new(
                    config.resources.heap_start,
                    config.resources.heap_size,
                )),
                sandbox: Arc::new(Sandbox::new(&config.sandbox.root_path)),
                processes: Arc::new(ProcessManager::new(terminal.clone())),
                network: Arc::new(NetworkManager::new()),
                time: Arc::new(TimeManager::new()),
                terminal: terminal.clone(),
                device: Arc::new(Mutex::new(crate::service::device::DeviceManager::new(
                    &config.vfio,
                    me.clone(),
                ))),
            }
        })
    }

    pub fn trigger_irq(&self, irq: usize) {
        let tbl = self.irq_manager.table.read().unwrap();
        if irq >= crate::kernel::irq::MAX_IRQS {
            return;
        }

        if let Some(cptr) = tbl[irq].cptr {
            if tbl[irq].enabled {
                let badge = tbl[irq].badge;
                println!("[hutch] irq: Notifying endpoint cptr {} with badge {}", cptr, badge);
                self.handle_endpoint_invocation(
                    cptr,
                    glenda::cap::ipcmethod::NOTIFY,
                    badge,
                    vec![],
                );
            }
        }
    }
}
