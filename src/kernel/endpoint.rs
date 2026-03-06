use crate::kernel::KernelState;
use glenda::cap::ipcmethod;

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// 表示一个正在等待 IPC 的线程
pub struct WaitingThread {
    pub tag: usize,
    pub mrs: Vec<usize>,
    pub signal: Arc<(Mutex<bool>, Condvar)>,
    pub reply: Arc<Mutex<Option<(usize, Vec<usize>)>>>,
}

pub struct Endpoint {
    pub inner: Mutex<EndpointInner>,
}

pub struct EndpointInner {
    pub receivers: VecDeque<WaitingThread>,
    pub senders: VecDeque<WaitingThread>,
    pub notifications: usize, // Bitmask or counter for pending notifications
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
        let ep_cap = self.cspace.get(cptr).expect("Invalid endpoint cap");

        let ep = match &ep_cap.cap_type {
            crate::kernel::capability::CapType::Endpoint => {
                self.resource.lock().unwrap().get_endpoint(cptr)
            }
            _ => return (u64::MAX as usize, vec![]),
        };

        let badge = ep_cap.badge.unwrap_or(0); // badge is Option<usize>

        let mut inner = ep.inner.lock().unwrap();

        match method {
            ipcmethod::NOTIFY => {
                // seL4 style: tag usually contains badge or badge from cap is or-ed
                let combined_badge = if badge != 0 { badge } else { tag };

                if let Some(receiver) = inner.receivers.pop_front() {
                    // Directly deliver to waiting receiver
                    let mut reply = receiver.reply.lock().unwrap();
                    // In notification, we return the badge in tag or MR0
                    *reply = Some((combined_badge, vec![]));
                    let (lock, cvar) = &*receiver.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                    (0, vec![])
                } else {
                    // Queue notification
                    inner.notifications |= combined_badge;
                    (0, vec![])
                }
            }
            ipcmethod::SEND => {
                if let Some(receiver) = inner.receivers.pop_front() {
                    // Direct transfer to waiting receiver
                    let mut reply = receiver.reply.lock().unwrap();
                    *reply = Some((tag, mrs));
                    let (lock, cvar) = &*receiver.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                    (0, vec![])
                } else {
                    // No receiver, block sender
                    let signal = Arc::new((Mutex::new(false), Condvar::new()));
                    let reply = Arc::new(Mutex::new(None));
                    let waiting = WaitingThread {
                        tag,
                        mrs: mrs.clone(),
                        signal: signal.clone(),
                        reply: reply.clone(),
                    };
                    inner.senders.push_back(waiting);
                    drop(inner);

                    // Wait for a receiver to pick up
                    let (lock, cvar) = &*signal;
                    let mut started = lock.lock().unwrap();
                    while !*started {
                        started = cvar.wait(started).unwrap();
                    }

                    let res = reply.lock().unwrap().take().unwrap_or((0, vec![]));
                    res
                }
            }
            ipcmethod::RECV => {
                if inner.notifications != 0 {
                    // Receive pending notification
                    let badge = inner.notifications;
                    inner.notifications = 0;
                    (badge, vec![])
                } else if let Some(sender) = inner.senders.pop_front() {
                    // Pull from waiting sender
                    let (lock, cvar) = &*sender.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                    (sender.tag, sender.mrs)
                } else {
                    // Block receiver
                    let signal = Arc::new((Mutex::new(false), Condvar::new()));
                    let reply = Arc::new(Mutex::new(None));
                    let waiting = WaitingThread {
                        tag: 0,
                        mrs: vec![],
                        signal: signal.clone(),
                        reply: reply.clone(),
                    };
                    inner.receivers.push_back(waiting);
                    drop(inner);

                    let (lock, cvar) = &*signal;
                    let mut started = lock.lock().unwrap();
                    while !*started {
                        started = cvar.wait(started).unwrap();
                    }

                    let res = reply.lock().unwrap().take().unwrap_or((0, vec![]));
                    res
                }
            }
            ipcmethod::CALL => {
                // CALL is atomic SEND then RECV for the same thread.
                // Simplified: delegate to SEND and handle the RECV part.
                // In hutch's single-handler-thread-per-client model,
                // we can just treat CALL as a blocking SEND that waits for a specific REPLY.

                // 1. Send to receiver
                if let Some(receiver) = inner.receivers.pop_front() {
                    let mut reply_slot = receiver.reply.lock().unwrap();
                    *reply_slot = Some((tag, mrs));
                    let (lock, cvar) = &*receiver.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                } else {
                    // No receiver, block as sender
                    let signal = Arc::new((Mutex::new(false), Condvar::new()));
                    let reply = Arc::new(Mutex::new(None));
                    inner.senders.push_back(WaitingThread {
                        tag,
                        mrs: mrs.clone(),
                        signal: signal.clone(),
                        reply: reply.clone(),
                    });
                    drop(inner);
                    let (lock, cvar) = &*signal;
                    let mut started = lock.lock().unwrap();
                    while !*started {
                        started = cvar.wait(started).unwrap();
                    }
                    // This sender was picked up by a RECV.
                    // Now it might need to wait for a REPLY if it's a CALL.
                }

                // 2. Wait for reply (Simplified for hosted)
                // In a real seL4 system, CALL sets a reply cap.
                // Here we just return 0 for now or implement a dedicated reply wait.
                (0, vec![])
            }
            _ => (u64::MAX as usize, vec![]),
        }
    }
}
