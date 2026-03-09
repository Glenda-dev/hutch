use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};
use std::thread;

pub struct TimeManager {
    boot_time: Instant,
}

impl TimeManager {
    pub fn new() -> Self {
        Self {
            boot_time: Instant::now(),
        }
    }

    pub fn get_time_ns(&self) -> usize {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as usize
    }

    pub fn get_uptime_ms(&self) -> usize {
        self.boot_time.elapsed().as_millis() as usize
    }

    pub fn sleep(&self, duration_ms: usize) {
        thread::sleep(Duration::from_millis(duration_ms));
    }
}
