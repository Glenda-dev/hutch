use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct TimeManager {
    boot_time: Instant,
}

impl TimeManager {
    pub fn new() -> Self {
        Self { boot_time: Instant::now() }
    }

    pub fn get_time_ns(&self) -> usize {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as usize
    }

    pub fn get_uptime_ms(&self) -> usize {
        self.boot_time.elapsed().as_millis() as usize
    }
}

impl glenda::interface::TimeService for &TimeManager {
    fn time_now(&mut self, _badge: glenda::ipc::Badge) -> Result<u64, glenda::error::Error> {
        Ok(self.get_time_ns() as u64)
    }

    fn mono_now(&mut self, _badge: glenda::ipc::Badge) -> Result<u64, glenda::error::Error> {
        Ok(self.get_uptime_ms() as u64 * 1000_000) // nanoseconds
    }

    fn sleep(&mut self, _badge: glenda::ipc::Badge, ms: usize) -> Result<(), glenda::error::Error> {
        thread::sleep(Duration::from_millis(ms as u64));
        Ok(())
    }

    fn adj_time(
        &mut self,
        _badge: glenda::ipc::Badge,
        _absolute_ns: u64,
        _drift_ppb: i64,
    ) -> Result<(), glenda::error::Error> {
        // Not supported in host OS
        Ok(())
    }
}
