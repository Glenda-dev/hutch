pub mod console;
pub mod cnode;
pub mod irq;
pub mod tcb;
pub mod vspace;

use crate::kernel::KernelState;
use crate::kernel::capability::CapType;
use glenda::ipc::utcb::UTCB;

impl KernelState {
    pub fn invoke_cap(&self, cptr: usize, method: usize, utcb_ptr: usize) -> usize {
        let utcb = unsafe { &mut *(utcb_ptr as *mut UTCB) };
        let tag = utcb.get_msg_tag();
        let mrs = utcb.get_mrs().to_vec();

        let (ret, out_mrs) = if let Some(cap) = self.cspace.read().unwrap().get(cptr).cloned() {
            match cap.cap_type {
                CapType::Console => self.handle_console_invocation(method, tag.0, mrs),
                CapType::TCB => self.handle_tcb_invocation(method, tag.0, mrs),
                CapType::CNode => self.handle_cnode_invocation(method, tag.0, mrs),
                CapType::VSpace => self.handle_vspace_invocation(method, tag.0, mrs),
CapType::Reply => {
                    if let Some(chan) = crate::kernel::endpoint::ACTIVE_REPLY.with(|r| r.borrow_mut().take()) {
                        let mut rep = chan.reply.lock().unwrap();
                        *rep = Some((tag.0, mrs));
                        let mut started = chan.signal.0.lock().unwrap();
                        *started = true;
                        chan.signal.1.notify_one();
                        (0, vec![])
                    } else {
                        eprintln!("[hutch] Reply called but no active reply channel");
                        (usize::MAX as usize, vec![])
                    }
                }
                CapType::Endpoint => {
                    let proto = tag.proto();
                    if proto == glenda::protocol::PROCESS_PROTO {
                        self.handle_process_emulation(method, tag.0, mrs)
                    } else if proto == glenda::protocol::RESOURCE_PROTO {
                        self.handle_resource_emulation(method, tag.0, mrs, utcb_ptr)
                    } else {
                        self.handle_endpoint_invocation(cptr, method, tag.0, mrs)
                    }
                }
                CapType::IrqHandler => {
                    self.handle_irq_invocation(cap.badge.unwrap_or(0), method, tag.0, mrs)
                }
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

    fn handle_process_emulation(&self, _method: usize, _tag: usize, mrs: Vec<usize>) -> (usize, Vec<usize>) {
        use glenda::protocol;
        let tag = glenda::ipc::MsgTag(_tag);
        if tag.label() == protocol::process::EXIT {
            let code = mrs[0];
            println!("[hutch] Process exited with code: {}", code);
            std::process::exit(code as i32);
        } else {
            (usize::MAX as usize, vec![])
        }
    }
    
    fn handle_resource_emulation(&self, _method: usize, _tag: usize, mrs: Vec<usize>, utcb_ptr: usize) -> (usize, Vec<usize>) {
        use glenda::protocol;
        let tag = glenda::ipc::MsgTag(_tag);
        let utcb = unsafe { &mut *(utcb_ptr as *mut UTCB) };

        if tag.label() == protocol::resource::SBRK {
            let incr = mrs[0] as isize;
            let mut res = self.resource.lock().unwrap();
            let old_brk = res.sbrk(incr);
            (old_brk, vec![])
        } else if tag.label() == protocol::resource::REGISTER_CAP {
            let cap = utcb.get_cap_transfer();
            let id = mrs[1];
            let mut res = self.resource.lock().unwrap();
            res.register_cap(id, cap.bits());
            (0, vec![])
        } else if tag.label() == protocol::resource::GET_CAP {
            let id = mrs[1];
            let res = self.resource.lock().unwrap();
            if let Some(cptr) = res.get_registered_cap(id) {
                utcb.set_recv_window(glenda::cap::CapPtr::from(cptr));
                (0, vec![])
            } else {
                (glenda::error::Error::NotFound as usize, vec![])
            }
        } else {
            (usize::MAX as usize, vec![])
        }
    }
}
