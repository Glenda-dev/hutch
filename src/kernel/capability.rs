pub use glenda::cap::CapType;
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum CapData {
    None,
    Untyped(RawFd),
    Frame(RawFd),
}

#[derive(Clone)]
pub struct Capability {
    pub cap_type: CapType,
    pub badge: Option<usize>,
    pub data: Arc<Mutex<CapData>>,
}

impl Capability {
    pub fn new(cap_type: CapType) -> Self {
        Self { cap_type, badge: None, data: Arc::new(Mutex::new(CapData::None)) }
    }

    pub fn with_badge(mut self, badge: usize) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn set_data(&self, data: CapData) {
        *self.data.lock().unwrap() = data;
    }

    pub fn get_data(&self) -> CapData {
        self.data.lock().unwrap().clone()
    }
}
