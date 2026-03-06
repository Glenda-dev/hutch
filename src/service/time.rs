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

    pub fn get_time_ns(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
    }

    pub fn get_uptime_ms(&self) -> u64 {
        self.boot_time.elapsed().as_millis() as u64
    }

    pub fn sleep(&self, duration_ms: u64) {
        thread::sleep(Duration::from_millis(duration_ms));
    }
}
