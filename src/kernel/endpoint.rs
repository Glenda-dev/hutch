use crate::kernel::KernelState;
use glenda::cap::ipcmethod;

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::cell::RefCell;

#[derive(Clone)]
pub struct ReplyChannel {
    pub signal: Arc<(Mutex<bool>, Condvar)>,
    pub reply: Arc<Mutex<Option<(usize, Vec<usize>)>>>,
}

thread_local! {
    pub static ACTIVE_REPLY: RefCell<Option<ReplyChannel>> = RefCell::new(None);
}

pub enum IPCKind {
    Send(Arc<(Mutex<bool>, Condvar)>),
    Call(ReplyChannel),
}

pub struct WaitingThread {
    pub tag: usize,
    pub mrs: Vec<usize>,
    pub kind: IPCKind,
}

pub struct Endpoint {
    pub inner: Mutex<EndpointInner>,
}

pub struct EndpointInner {
    pub receivers: VecDeque<Arc<(Mutex<bool>, Condvar, Mutex<Option<(usize, Vec<usize>, Option<ReplyChannel>)>>)>>,
    pub senders: VecDeque<WaitingThread>,
    pub notifications: usize,
}

impl Endpoint {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(EndpointInner {
                receivers: VecDeque::new(),
                senders: VecDeque::new(),
                notifications: 0,
            }),
        }
    }
}

impl KernelState {
    pub fn handle_endpoint_invocation(
        &self,
        cptr: usize,
        method: usize,
        tag: usize,
        mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        let ep_cap = self.cspace.read().unwrap().get(cptr).cloned().unwrap_or(crate::kernel::capability::Capability::new(crate::kernel::capability::CapType::Empty));
        
        let ep = if ep_cap.cap_type == crate::kernel::capability::CapType::Endpoint {
            self.resource.lock().unwrap().get_endpoint(cptr)
        } else {
            return (usize::MAX as usize, vec![]);
        };

        let badge = ep_cap.badge.unwrap_or(0);
        let mut inner = ep.inner.lock().unwrap();

        match method {
            ipcmethod::NOTIFY => {
                let combined_badge = if badge != 0 { badge } else { tag };
                if let Some(receiver) = inner.receivers.pop_front() {
                    let mut rep = receiver.2.lock().unwrap();
                    *rep = Some((combined_badge, vec![], None));
                    let mut started = receiver.0.lock().unwrap();
                    *started = true;
                    receiver.1.notify_one();
                } else {
                    inner.notifications |= combined_badge;
                }
                (0, vec![])
            }
            ipcmethod::SEND => {
                let sent_tag = if badge != 0 { tag | (badge << 32) } else { tag };
                if let Some(receiver) = inner.receivers.pop_front() {
                    let mut rep = receiver.2.lock().unwrap();
                    *rep = Some((sent_tag, mrs, None));
                    let mut started = receiver.0.lock().unwrap();
                    *started = true;
                    receiver.1.notify_one();
                    (0, vec![])
                } else {
                    let signal = Arc::new((Mutex::new(false), Condvar::new()));
                    inner.senders.push_back(WaitingThread {
                        tag: sent_tag,
                        mrs,
                        kind: IPCKind::Send(signal.clone()),
                    });
                    drop(inner);
                    
                    let mut started = signal.0.lock().unwrap();
                    while !*started {
                        started = signal.1.wait(started).unwrap();
                    }
                    (0, vec![])
                }
            }
            ipcmethod::RECV => {
                if inner.notifications != 0 {
                    let badge = inner.notifications;
                    inner.notifications = 0;
                    (badge, vec![])
                } else if let Some(sender) = inner.senders.pop_front() {
                    match sender.kind {
                        IPCKind::Send(sig) => {
                            let mut started = sig.0.lock().unwrap();
                            *started = true;
                            sig.1.notify_one();
                        }
                        IPCKind::Call(chan) => {
                            ACTIVE_REPLY.with(|r| *r.borrow_mut() = Some(chan));
                        }
                    }
                    (sender.tag, sender.mrs)
                } else {
                    let signal = Arc::new((Mutex::new(false), Condvar::new(), Mutex::new(None)));
                    inner.receivers.push_back(signal.clone());
                    drop(inner);

                    let mut started = signal.0.lock().unwrap();
                    while !*started {
                        started = signal.1.wait(started).unwrap();
                    }
                    
                    let (res_tag, res_mrs, opt_chan) = signal.2.lock().unwrap().take().unwrap();
                    if let Some(chan) = opt_chan {
                        ACTIVE_REPLY.with(|r| *r.borrow_mut() = Some(chan));
                    }
                    (res_tag, res_mrs)
                }
            }
            ipcmethod::CALL => {
                let sent_tag = if badge != 0 { tag | (badge << 32) } else { tag };
                let chan = ReplyChannel {
                    signal: Arc::new((Mutex::new(false), Condvar::new())),
                    reply: Arc::new(Mutex::new(None)),
                };
                
                if let Some(receiver) = inner.receivers.pop_front() {
                    let mut rep = receiver.2.lock().unwrap();
                    *rep = Some((sent_tag, mrs, Some(chan.clone())));
                    let mut started = receiver.0.lock().unwrap();
                    *started = true;
                    receiver.1.notify_one();
                } else {
                    inner.senders.push_back(WaitingThread {
                        tag: sent_tag,
                        mrs,
                        kind: IPCKind::Call(chan.clone()),
                    });
                }
                drop(inner);
                
                let mut started = chan.signal.0.lock().unwrap();
                while !*started {
                    started = chan.signal.1.wait(started).unwrap();
                }
                
                let res = chan.reply.lock().unwrap().take().unwrap_or((0, vec![]));
                res
            }
            _ => (usize::MAX as usize, vec![]),
        }
    }
}
