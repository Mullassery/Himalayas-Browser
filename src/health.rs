use std::time::Instant;

#[derive(Clone, Debug)]
pub struct HealthMonitor {
    start_time: Instant,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub fn is_healthy(&self) -> bool {
        true
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}
