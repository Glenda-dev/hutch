#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapType {
    Empty,
    Untyped,
    TCB,
    Endpoint,
    Reply,
    Frame {
        addr: usize,
        size: usize,
    },
    PageTable,
    CNode,
    IrqHandler,
    Kernel,
    VSpace,
    Console,
    Monitor,
}

pub struct Capability {
    pub cap_type: CapType,
    pub badge: Option<usize>,
}

impl Capability {
    pub fn new(cap_type: CapType) -> Self {
        Self { cap_type, badge: None }
    }

    pub fn with_badge(mut self, badge: usize) -> Self {
        self.badge = Some(badge);
        self
    }
}
