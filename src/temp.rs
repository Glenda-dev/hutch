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

/// 表示一个正在等待 IPC 的线程
pub struct WaitingThread {
    pub tag: usize,
    pub mrs: Vec<usize>,
    pub channel: ReplyChannel,
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
        let ep_cap = self.cspace.read().unwrap().get(cptr).cloned().expect("Invalid endpoint cap");

        let ep = match &ep_cap.cap_type {
            crate::kernel::capability::CapType::Endpoint => {
                self.resource.lock().unwrap().get_endpoint(cptr)
            }
            _ => return (usize::MAX as usize, vec![]),
        };

        let badge = ep_cap.badge.unwrap_or(0); // badge is Option<usize>

        let mut inner = ep.inner.lock().unwrap();

        match method {
            ipcmethod::NOTIFY => {
                // seL4 style: tag usually contains badge or badge from cap is or-ed
                let combined_badge = if badge != 0 { badge } else { tag };

                if let Some(receiver) = inner.receivers.pop_front() {
                    // Directly deliver to waiting receiver
                    let mut reply = receiver.channel.reply.lock().unwrap();
                    // In notification, we return the badge in tag or MR0
                    *reply = Some((combined_badge, vec![]));
                    let (lock, cvar) = &*receiver.channel.signal;
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
                    let mut reply = receiver.channel.reply.lock().unwrap();
                    *reply = Some((tag, mrs));
                    let (lock, cvar) = &*receiver.channel.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                    (0, vec![])
                } else {
                    // No receiver, block sender
                    let signal = Arc::new((Mutex::new(false), Condvar::new()));
                    let reply = Arc::new(Mutex::new(None));
                    let channel = ReplyChannel { signal: signal.clone(), reply: reply.clone() };
                    let waiting = WaitingThread {
                        tag,
                        mrs: mrs.clone(),
                        channel,
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
                    // Because RECV just consumes SEND or CALL, we must save its reply channel for later if it expects a reply.
                    // Actually, if sender was CALL, we store it to ACTIVE_REPLY. If it was SEND, it wouldn't care, but storing is fine.
                    ACTIVE_REPLY.with(|r| *r.borrow_mut() = Some(sender.channel.clone()));
                    
                    let (lock, cvar) = &*sender.channel.signal;
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
                        channel: ReplyChannel { signal: signal.clone(), reply: reply.clone() },
                    };
                    inner.receivers.push_back(waiting);
                    drop(inner);

                    let (lock, cvar) = &*signal;
                    let mut started = lock.lock().unwrap();
                    while !*started {
                        started = cvar.wait(started).unwrap();
                    }

                    // A sender has picked us up and furnished the reply
                    // If it was a CALL, the sender has already completed SEND part. We should set its channel to ACTIVE_REPLY so we can reply to it later.
                    let (res_tag, res_mrs, incoming_channel) = reply.lock().unwrap().take().unwrap_or((0, vec![], None));
                    if let Some(chan) = incoming_channel {
                        ACTIVE_REPLY.with(|r| *r.borrow_mut() = Some(chan));
                    }
                    (res_tag, res_mrs)
                }
            }
            ipcmethod::CALL => {
                // 1. Send to receiver
                let signal = Arc::new((Mutex::new(false), Condvar::new()));
                let reply = Arc::new(Mutex::new(None));
                let my_channel = ReplyChannel { signal: signal.clone(), reply: reply.clone() };

                if let Some(receiver) = inner.receivers.pop_front() {
                    let mut reply_slot = receiver.channel.reply.lock().unwrap();
                    // Provide our channel to the receiver so they can reply to us
                    *reply_slot = Some((tag, mrs, Some(my_channel)));
                    let (lock, cvar) = &*receiver.channel.signal;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                } else {
                    // No receiver, block as sender
                    inner.senders.push_back(WaitingThread {
                        tag,
                        mrs: mrs.clone(),
                        channel: my_channel,
                    });
                    drop(inner);
                }

                // Wait for the REPLY
                // Note that in SEND, we waited for a receiver to pick us up. In CALL, we wait for the reply.
                // The receiver will not signal us until it calls REPLY. Wait! In RECV, we did signal the sender immediately!
                // Ah! We shouldn't signal immediately if it's CALL, or we should use TWO condvars?
                // Actually, if we just wait on `my_channel.signal`, whoever finishes our request will signal it!
                // Let's rely on the receiver to signal us when they call REPLY. The RECV should *not* signal CALLers immediately!

            _ => (usize::MAX as usize, vec![]),
        }
    }
}
