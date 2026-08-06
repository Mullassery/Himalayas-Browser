use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct MetricsCollector {
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn memory_mb(&self) -> u64 {
        0 // TODO: Implement memory usage tracking
    }

    pub fn record_request(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }
}
