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
    pub receivers: VecDeque<WaitingThread>,
    pub senders: VecDeque<WaitingThread>,
}

impl Endpoint {
    pub fn new() -> Self {
        Self { receivers: VecDeque::new(), senders: VecDeque::new() }
    }
}

impl KernelState {
    pub fn handle_endpoint_invocation(
        &self,
        _cptr: usize,
        method: usize,
        _tag: usize,
        _mrs: Vec<usize>,
    ) -> (usize, Vec<usize>) {
        // 在 Hutch 中，Endpoint 模拟通常需要跨连接同步。
        // 为了简化，目前先实现同一个进程内的 Endpoint 仿真逻辑。
        // 注意：真正的多进程 IPC 需要 Hutch 在其会话管理层处理。

        match method {
            ipcmethod::SEND => {
                // TODO: 真正的阻塞逻辑需要挂起当前处理循环
                (0, vec![])
            }
            ipcmethod::RECV => (0, vec![]),
            ipcmethod::CALL => (0, vec![]),
            _ => (u64::MAX as usize, vec![]),
        }
    }
}
