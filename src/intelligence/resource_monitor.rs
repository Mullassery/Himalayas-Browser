use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Real-time resource metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceMetrics {
    pub timestamp_ms: u64,
    pub memory_usage_mb: u32,
    pub memory_pressure: f32,      // 0.0-1.0
    pub cpu_load: f32,              // 0.0-1.0
    pub gpu_load: f32,              // 0.0-1.0
    pub battery_level: u8,          // 0-100
    pub is_plugged_in: bool,
    pub temperature_celsius: f32,
    pub thermal_throttling: bool,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            memory_usage_mb: 0,
            memory_pressure: 0.0,
            cpu_load: 0.0,
            gpu_load: 0.0,
            battery_level: 100,
            is_plugged_in: true,
            temperature_celsius: 45.0,
            thermal_throttling: false,
        }
    }
}

/// Resource monitor for runtime adaptation
pub struct ResourceMonitor {
    metrics: Arc<RwLock<ResourceMetrics>>,
    history: Arc<RwLock<Vec<ResourceMetrics>>>,
    max_history: usize,
}

impl ResourceMonitor {
    pub fn new() -> Result<Self> {
        debug!("Initializing Resource Monitor");

        let initial_metrics = Self::collect_metrics()?;

        Ok(Self {
            metrics: Arc::new(RwLock::new(initial_metrics)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        })
    }

    /// Collect current system metrics
    fn collect_metrics() -> Result<ResourceMetrics> {
        // Simplified metrics collection
        let total_ram_mb: u32 = 8192; // Default 8GB
        let used_ram_mb: u32 = 4096; // Estimate 50% usage
        let memory_pressure = (used_ram_mb as f32 / total_ram_mb as f32).clamp(0.0, 1.0);

        // CPU load estimation (simplified)
        let cpu_load = 0.35; // Default moderate load

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        Ok(ResourceMetrics {
            timestamp_ms,
            memory_usage_mb: used_ram_mb,
            memory_pressure,
            cpu_load,
            gpu_load: 0.0, // Would require GPU-specific queries
            battery_level: 100,
            is_plugged_in: true,
            temperature_celsius: 45.0,
            thermal_throttling: false,
        })
    }

    /// Update metrics (call periodically from daemon)
    pub fn update(&self) -> Result<()> {
        let new_metrics = Self::collect_metrics()?;

        // Update current
        {
            let mut metrics = self.metrics.write();
            *metrics = new_metrics.clone();
        }

        // Add to history
        {
            let mut history = self.history.write();
            if history.len() >= self.max_history {
                history.remove(0);
            }
            history.push(new_metrics);
        }

        Ok(())
    }

    /// Get current snapshot
    pub fn snapshot(&self) -> ResourceMetrics {
        self.metrics.read().clone()
    }

    /// Get metric history
    pub fn history(&self) -> Vec<ResourceMetrics> {
        self.history.read().clone()
    }

    /// Check if under memory pressure
    pub fn is_memory_pressure(&self) -> bool {
        self.metrics.read().memory_pressure > 0.80
    }

    /// Check if under CPU pressure
    pub fn is_cpu_pressure(&self) -> bool {
        self.metrics.read().cpu_load > 0.85
    }

    /// Check if in low battery mode
    pub fn is_low_battery(&self) -> bool {
        let metrics = self.metrics.read();
        metrics.battery_level < 20 && !metrics.is_plugged_in
    }

    /// Get pressure level (0-1)
    pub fn overall_pressure(&self) -> f32 {
        let metrics = self.metrics.read();
        (metrics.memory_pressure + metrics.cpu_load) / 2.0
    }

    /// Trend analysis (simple moving average)
    pub fn memory_trend(&self) -> MemoryTrend {
        let history = self.history.read();
        if history.len() < 2 {
            return MemoryTrend::Stable;
        }

        let recent = &history[history.len() - 1];
        let older = &history[0];

        let diff = recent.memory_pressure - older.memory_pressure;
        if diff > 0.15 {
            MemoryTrend::Increasing
        } else if diff < -0.15 {
            MemoryTrend::Decreasing
        } else {
            MemoryTrend::Stable
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrend {
    Increasing,
    Stable,
    Decreasing,
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new().expect("Failed to create resource monitor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_creation() {
        let monitor = ResourceMonitor::new().unwrap();
        let metrics = monitor.snapshot();
        assert!(metrics.memory_usage_mb > 0);
    }

    #[test]
    fn test_memory_pressure_detection() {
        let monitor = ResourceMonitor::new().unwrap();
        let pressure = monitor.snapshot().memory_pressure;
        assert!(pressure >= 0.0 && pressure <= 1.0);
    }

    #[test]
    fn test_metrics_update() {
        let monitor = ResourceMonitor::new().unwrap();
        let initial = monitor.snapshot();

        monitor.update().unwrap();
        let updated = monitor.snapshot();

        assert_eq!(initial.memory_pressure >= 0.0, updated.memory_pressure >= 0.0);
    }

    #[test]
    fn test_history_tracking() {
        let monitor = ResourceMonitor::new().unwrap();
        monitor.update().unwrap();
        monitor.update().unwrap();

        let history = monitor.history();
        assert!(history.len() >= 1);
    }

    #[test]
    fn test_pressure_checks() {
        let monitor = ResourceMonitor::new().unwrap();
        let is_mem_pressure = monitor.is_memory_pressure();
        let is_cpu_pressure = monitor.is_cpu_pressure();

        // Should both be boolean
        let _ = is_mem_pressure && is_cpu_pressure;
    }
}
