pub mod capability;
pub mod cspace;

use crate::config::Config;
use crate::kernel::capability::{CapType, Capability};
use crate::kernel::cspace::CSpace;
use crate::service::fs::Sandbox;
use crate::service::resource::ResourceManager;
use glenda::cap::ipcmethod;
use glenda::ipc::utcb::UTCB;
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
    pub cspace: CSpace,
    pub resource: Mutex<ResourceManager>,
    pub sandbox: Arc<Sandbox>,
}

impl KernelState {
    pub fn new(config: Config) -> Self {
        let mut cspace = CSpace::new();
        // Setup well-known caps
        cspace.insert(CSPACE_CPTR, Capability::new(CapType::CNode));
        cspace.insert(VSPACE_CPTR, Capability::new(CapType::VSpace));
        cspace.insert(TCB_CPTR, Capability::new(CapType::TCB));
        cspace.insert(MONITOR_CPTR, Capability::new(CapType::Monitor));
        cspace.insert(CONSOLE_CPTR, Capability::new(CapType::Console));
        cspace.insert(REPLY_CPTR, Capability::new(CapType::Reply));
        cspace.insert(ENDPOINT_CPTR, Capability::new(CapType::Endpoint));

        Self {
            cspace,
            resource: Mutex::new(ResourceManager {
                heap_start: config.resources.heap_start,
                heap_size: config.resources.heap_size,
                brk: config.resources.heap_start,
            }),
            sandbox: Arc::new(Sandbox::new(&config.sandbox.root_path)),
        }
    }

    pub fn invoke_cap(&self, cptr: usize, method: usize, utcb_ptr: usize) -> usize {
        let utcb = unsafe { &mut *(utcb_ptr as *mut UTCB) };
        let tag = utcb.get_msg_tag();
        let mrs = utcb.get_mrs().to_vec();

        let (ret, out_mrs) = if let Some(cap) = self.cspace.get(cptr) {
            match cap.cap_type {
                CapType::Monitor => self.handle_monitor_invocation(method, tag.0, mrs),
                CapType::Console => self.handle_console_invocation(method, tag.0, mrs),
                CapType::TCB => self.handle_tcb_invocation(method, tag.0, mrs),
                CapType::CNode => self.handle_cnode_invocation(method, tag.0, mrs),
                CapType::VSpace => self.handle_vspace_invocation(method, tag.0, mrs),
                CapType::Endpoint => self.handle_endpoint_invocation(cptr, method, tag.0, mrs),
                _ => {
                    eprintln!(
                        "[hutch] Unimplemented cap type invocation: {:?} (cptr: {})",
                        cap.cap_type, cptr
                    );
                    (u64::MAX as usize, vec![])
                }
            }
        } else {
            eprintln!("[hutch] Invalid cap pointer: {}", cptr);
            (u64::MAX as usize, vec![])
        };

        for (i, &val) in out_mrs.iter().enumerate() {
            utcb.set_mr(i, val);
        }
        ret
    }

    fn handle_console_invocation(
        &self,
        method: usize,
        tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::ipc::MsgTag;
        let _msg_tag = MsgTag(tag);
        // Console typically handles generic IPC SEND for print
        if method == ipcmethod::SEND || method == ipcmethod::CALL {
            // Placeholder: Print from MRs
            for &val in &mrs {
                if val == 0 {
                    break;
                }
                print!("{}", val as u8 as char);
            }
        }
        (0, vec![])
    }

    fn handle_tcb_invocation(
        &self,
        _method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        // TCB configure, etc. In hosted mode, we might just track state
        (0, vec![])
    }

    fn handle_cnode_invocation(
        &self,
        method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::cap::cnodemethod;
        match method {
            cnodemethod::MINT | cnodemethod::COPY | cnodemethod::MOVE => {
                // In hosted, we just track this in the local CSpace map.
                (0, vec![])
            }
            cnodemethod::DELETE | cnodemethod::REVOKE => (0, vec![]),
            _ => (u64::MAX as usize, vec![]),
        }
    }

    fn handle_vspace_invocation(
        &self,
        method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::cap::vspacemethod;
        match method {
            vspacemethod::MAP | vspacemethod::MAP_TABLE | vspacemethod::SETUP => {
                // Return success as address space is host-managed
                (0, vec![])
            }
            _ => (0, vec![]),
        }
    }

    fn handle_monitor_invocation(
        &self,
        method: usize,
        tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        use glenda::ipc::MsgTag;
        let msg_tag = MsgTag(tag);
        let proto = msg_tag.proto();
        let label = msg_tag.label();

        match method {
            ipcmethod::CALL | ipcmethod::SEND => {
                match proto {
                    glenda::protocol::RESOURCE_PROTO => {
                        // Resource Service (e.g., SBRK, Memory Info)
                        match label {
                            glenda::protocol::resource::SBRK => {
                                let mut res = self.resource.lock().unwrap();
                                let ret = res.sbrk(mrs[0] as isize);
                                (ret, vec![])
                            }
                            _ => (0, vec![]),
                        }
                    }
                    glenda::protocol::PROCESS_PROTO => {
                        // Process Service (e.g., EXIT, SPAWN)
                        match label {
                            glenda::protocol::process::EXIT => {
                                println!("[hutch] Process exit requested with code: {}", mrs[0]);
                                std::process::exit(mrs[0] as i32);
                            }
                            _ => (0, vec![]),
                        }
                    }
                    glenda::protocol::FS_PROTO => {
                        // FS Service mapping
                        (0, vec![])
                    }
                    _ => {
                        eprintln!("[hutch] Monitor: Unhandled protocol {}", proto);
                        (0, vec![])
                    }
                }
            }
            _ => (u64::MAX as usize, vec![]),
        }
    }
}
pub mod endpoint;
