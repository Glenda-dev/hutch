pub mod capability;
pub mod cspace;

use crate::config::Config;
use crate::kernel::capability::{CapType, Capability};
use crate::kernel::cspace::CSpace;
use crate::service::fs::Sandbox;
use crate::service::network::NetworkManager;
use crate::service::process::ProcessManager;
use crate::service::resource::ResourceManager;
use crate::service::terminal::TerminalManager;
use crate::service::time::TimeManager;
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
    pub processes: Arc<ProcessManager>,
    pub network: Arc<NetworkManager>,
    pub time: Arc<TimeManager>,
    pub terminal: Arc<TerminalManager>,
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
            resource: Mutex::new(ResourceManager::new(
                config.resources.heap_start,
                config.resources.heap_size,
            )),
            sandbox: Arc::new(Sandbox::new(&config.sandbox.root_path)),
            processes: Arc::new(ProcessManager::new()),
            network: Arc::new(NetworkManager::new()),
            time: Arc::new(TimeManager::new()),
            terminal: Arc::new(TerminalManager::new()),
        }
    }

    pub fn invoke_cap(&self, cptr: usize, method: usize, utcb_ptr: usize) -> usize {
        let utcb = unsafe { &mut *(utcb_ptr as *mut UTCB) };
        let tag = utcb.get_msg_tag();
        let mrs = utcb.get_mrs().to_vec();

        let (ret, out_mrs) = if let Some(cap) = self.cspace.get(cptr) {
            match cap.cap_type {
                CapType::Monitor => self.handle_monitor_invocation(method, tag.0, mrs, utcb),
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
                    (usize::MAX as usize, vec![])
                }
            }
        } else {
            eprintln!("[hutch] Invalid cap pointer: {}", cptr);
            (usize::MAX as usize, vec![])
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
            _ => (usize::MAX as usize, vec![]),
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
        utcb: &mut UTCB,
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
                            glenda::protocol::resource::ALLOC => {
                                // Alloc generic capability (e.g. Endpoint, Reply, etc.)
                                // Simple simulation: just create a new cap of that type
                                // In a real system this would involve untyped memory allocation

                                // For now, we only need to handle a few types for simulation

                                static mut NEXT_OBJ_CPTR: usize = 2000;
                                let cptr = unsafe {
                                    let val = NEXT_OBJ_CPTR;
                                    NEXT_OBJ_CPTR += 1;
                                    val
                                };

                                // Note: In this simulation environment, we'd need a way
                                // to manage the cptr space across the whole hutch instance.
                                // For simplicity, we just return the cptr.
                                (0, vec![cptr])
                            }
                            glenda::protocol::resource::GET_CAP => {
                                let cap_type_val = mrs[0];
                                let id = mrs[1];

                                let cap_type_res =
                                    glenda::protocol::resource::ResourceType::from(cap_type_val);
                                let res = self.resource.lock().unwrap();

                                // For Endpoint resources, we search if they are registered externally
                                if cap_type_res
                                    == glenda::protocol::resource::ResourceType::Endpoint
                                {
                                    if let Some(cptr) = res.get_registered_cap(id) {
                                        return (0, vec![cptr]);
                                    }

                                    // Default/Internal implementations for endpoints
                                    match id {
                                        glenda::protocol::resource::FS_ENDPOINT => {
                                            (0, vec![ENDPOINT_CPTR])
                                        }
                                        _ => (0, vec![0]),
                                    }
                                } else if cap_type_res
                                    == glenda::protocol::resource::ResourceType::Console
                                {
                                    // Use well-known console directly
                                    (0, vec![CONSOLE_CPTR])
                                } else {
                                    (0, vec![0])
                                }
                            }
                            glenda::protocol::resource::REGISTER_CAP => {
                                let cap_type_val = mrs[0];
                                let id = mrs[1];
                                // We also expect the capability to be passed in utcb cap_transfer
                                // In the simulated environment, it might just be mrs[2] or through cap transfer.
                                // Let's check how many mrs we get.
                                // If it's an endpoint, register it:
                                if glenda::protocol::resource::ResourceType::from(cap_type_val)
                                    == glenda::protocol::resource::ResourceType::Endpoint
                                {
                                    let cptr = if mrs.len() > 2 { mrs[2] } else { 0 }; // Just safely fallback
                                    let mut res = self.resource.lock().unwrap();
                                    res.register_cap(id, cptr);
                                }
                                (0, vec![])
                            }
                            glenda::protocol::resource::DMA_ALLOC => {
                                let pages = mrs[0];
                                let size = pages * 4096; // Page size
                                // Use a temporary file and mmap to support named sharing if needed,
                                // or just anonymous for simple parent-child (or shared hutch state)
                                // In hosted mode, we can use shm_open or unique temp files.

                                let file_name =
                                    format!("/tmp/glenda_shm_{}", unsafe { libc::rand() });
                                let fd = unsafe {
                                    libc::shm_open(
                                        std::ffi::CString::new(file_name.clone()).unwrap().as_ptr(),
                                        libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
                                        0o666,
                                    )
                                };

                                if fd < 0 {
                                    return (usize::MAX as usize, vec![]);
                                }

                                unsafe { libc::ftruncate(fd, size as libc::off_t) };

                                let ptr = unsafe {
                                    libc::mmap(
                                        std::ptr::null_mut(),
                                        size,
                                        libc::PROT_READ | libc::PROT_WRITE,
                                        libc::MAP_SHARED,
                                        fd,
                                        0,
                                    )
                                };

                                if ptr == libc::MAP_FAILED {
                                    unsafe { libc::close(fd) };
                                    return (usize::MAX as usize, vec![]);
                                }

                                // In hutch simulation, we store the host path and id for potential multi-process sharing

                                static mut NEXT_FRAME_CPTR: usize = 1000;
                                let cptr = unsafe {
                                    let val = NEXT_FRAME_CPTR;
                                    NEXT_FRAME_CPTR += 1;
                                    val
                                };

                                (0, vec![cptr, ptr as usize, fd as usize])
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
                            glenda::protocol::process::SPAWN => {
                                // Extract path from UTCB
                                let path_ptr = utcb.get_buffer_ptr();
                                let path_len = utcb.get_buffer_size();
                                let path_bytes =
                                    unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
                                let path_str = std::str::from_utf8(path_bytes)
                                    .unwrap_or("")
                                    .trim_matches('\0');

                                match self.processes.spawn(path_str, vec![]) {
                                    Ok(pid) => (pid, vec![]),
                                    Err(_) => (usize::MAX as usize, vec![]),
                                }
                            }
                            _ => (0, vec![]),
                        }
                    }
                    glenda::protocol::FS_PROTO => {
                        // FS Service mapping
                        match label {
                            glenda::protocol::fs::OPEN => {
                                let flags =
                                    glenda::protocol::fs::OpenFlags::from_bits_truncate(mrs[0]);
                                // Path is in UTCB IPC buffer
                                let path_ptr = utcb.get_buffer_ptr();
                                let path_len = utcb.get_buffer_size();
                                let path_bytes =
                                    unsafe { std::slice::from_raw_parts(path_ptr, path_len) };
                                let path_str = std::str::from_utf8(path_bytes)
                                    .unwrap_or("")
                                    .trim_matches('\0');

                                match self.sandbox.open(path_str, flags) {
                                    Ok(_file) => {
                                        // In a real hutch, we would store the File in a table and return a badged endpoint
                                        // For now, return success
                                        (0, vec![REPLY_CPTR])
                                    }
                                    Err(e) => {
                                        eprintln!("[hutch] FS Open failed for {}: {}", path_str, e);
                                        (usize::MAX as usize, vec![])
                                    }
                                }
                            }
                            glenda::protocol::fs::SETUP_IOURING => {
                                // mrs[0]: size, mrs[1]: frame_cptr
                                let frame_cptr = mrs[1];

                                if let Some(cap) = self.cspace.get(frame_cptr) {
                                    if let CapType::Frame { addr, size: _ } = cap.cap_type {
                                        let mut res = self.resource.lock().unwrap();
                                        let emulator = crate::service::uring::UringEmulator::new(
                                            addr as *mut u8,
                                        );
                                        res.register_uring(frame_cptr, emulator);
                                        (0, vec![])
                                    } else {
                                        (usize::MAX as usize, vec![])
                                    }
                                } else {
                                    (usize::MAX as usize, vec![])
                                }
                            }
                            glenda::protocol::fs::PROCESS_IOURING => {
                                // mrs[0]: frame_cptr
                                let frame_cptr = mrs[0];
                                let mut res = self.resource.lock().unwrap();
                                match res.process_uring(frame_cptr) {
                                    Ok(_) => (0, vec![]),
                                    Err(_) => (usize::MAX as usize, vec![]),
                                }
                            }
                            glenda::protocol::fs::READ_SYNC => {
                                // Implement using self.sandbox
                                (0, vec![])
                            }
                            _ => (0, vec![]),
                        }
                    }
                    _ => {
                        eprintln!("[hutch] Monitor: Unhandled protocol {}", proto);
                        (0, vec![])
                    }
                }
            }
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
pub mod endpoint;
