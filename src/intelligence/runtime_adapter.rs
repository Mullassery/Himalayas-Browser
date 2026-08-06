use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::intelligence::profile_manager::AdaptiveProfile;
use crate::intelligence::resource_monitor::{ResourceMetrics, MemoryTrend};

/// Continuous runtime adaptation
pub struct RuntimeAdapter {
    adaptation_history: Arc<RwLock<Vec<AdaptationEvent>>>,
    aggressive_adaptations: Arc<RwLock<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationEvent {
    pub timestamp_ms: u64,
    pub adaptation_type: AdaptationType,
    pub reason: String,
    pub applied: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AdaptationType {
    ReduceAnimations,
    SuspendBackgroundTasks,
    ReduceAiContext,
    FreezeInactiveTabs,
    ReduceCacheSize,
    SuspendIndexing,
    ReduceGpuLoad,
    RestrictNetworkActivity,
    ReduceVisionProcessing,
    ResumeFeatures,
}

impl RuntimeAdapter {
    pub fn new() -> Self {
        debug!("Initializing Runtime Adapter");

        Self {
            adaptation_history: Arc::new(RwLock::new(Vec::new())),
            aggressive_adaptations: Arc::new(RwLock::new(0)),
        }
    }

    /// Analyze metrics and suggest adaptations
    pub fn analyze_metrics(&self, metrics: &ResourceMetrics, trend: MemoryTrend) -> Vec<AdaptationType> {
        let mut adaptations = vec![];

        // Memory pressure adaptations
        if metrics.memory_pressure > 0.90 {
            adaptations.push(AdaptationType::FreezeInactiveTabs);
            adaptations.push(AdaptationType::SuspendIndexing);
            adaptations.push(AdaptationType::ReduceAiContext);
        } else if metrics.memory_pressure > 0.80 {
            adaptations.push(AdaptationType::ReduceCacheSize);
            adaptations.push(AdaptationType::SuspendBackgroundTasks);
        } else if metrics.memory_pressure > 0.70 {
            adaptations.push(AdaptationType::ReduceAnimations);
        }

        // CPU pressure adaptations
        if metrics.cpu_load > 0.90 {
            adaptations.push(AdaptationType::ReduceVisionProcessing);
            adaptations.push(AdaptationType::SuspendBackgroundTasks);
        } else if metrics.cpu_load > 0.80 {
            adaptations.push(AdaptationType::ReduceGpuLoad);
        }

        // Thermal throttling adaptations
        if metrics.thermal_throttling {
            warn!("Thermal throttling detected");
            adaptations.push(AdaptationType::ReduceGpuLoad);
            adaptations.push(AdaptationType::SuspendBackgroundTasks);
        }

        // Battery-aware adaptations
        if metrics.battery_level < 10 && !metrics.is_plugged_in {
            warn!("Critical battery level");
            adaptations.push(AdaptationType::RestrictNetworkActivity);
            adaptations.push(AdaptationType::ReduceAnimations);
        } else if metrics.battery_level < 20 && !metrics.is_plugged_in {
            adaptations.push(AdaptationType::ReduceGpuLoad);
        }

        // Trend-based adaptations
        if trend == MemoryTrend::Increasing {
            debug!("Memory trend: increasing - proactive adaptation");
            adaptations.push(AdaptationType::ReduceCacheSize);
        }

        adaptations.sort_by_key(|a| self.adaptation_priority(a));
        adaptations.dedup();

        adaptations
    }

    /// Get priority of adaptation (lower = higher priority)
    fn adaptation_priority(&self, adaptation: &AdaptationType) -> u32 {
        match adaptation {
            AdaptationType::FreezeInactiveTabs => 1,
            AdaptationType::SuspendBackgroundTasks => 2,
            AdaptationType::ReduceAiContext => 3,
            AdaptationType::SuspendIndexing => 4,
            AdaptationType::ReduceCacheSize => 5,
            AdaptationType::ReduceVisionProcessing => 6,
            AdaptationType::ReduceGpuLoad => 7,
            AdaptationType::ReduceAnimations => 8,
            AdaptationType::RestrictNetworkActivity => 9,
            AdaptationType::ResumeFeatures => 10,
        }
    }

    /// Log adaptation event
    pub fn log_adaptation(&self, adaptation: AdaptationType, reason: String, applied: bool) {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let event = AdaptationEvent {
            timestamp_ms,
            adaptation_type: adaptation,
            reason: reason.clone(),
            applied,
        };

        let mut history = self.adaptation_history.write();
        if history.len() >= 100 {
            history.remove(0);
        }
        history.push(event);

        if applied {
            info!("Applied adaptation: {:?} - {}", adaptation, reason);
        } else {
            debug!("Skipped adaptation: {:?} - {}", adaptation, reason);
        }
    }

    /// Check if aggressive adaptations needed
    pub fn is_critical_state(&self, metrics: &ResourceMetrics) -> bool {
        metrics.memory_pressure > 0.95
            || (metrics.cpu_load > 0.95 && metrics.memory_pressure > 0.80)
            || metrics.thermal_throttling
            || (metrics.battery_level < 5 && !metrics.is_plugged_in)
    }

    /// Recovery actions when returning to normal
    pub fn recovery_adaptations(&self, metrics: &ResourceMetrics) -> Vec<AdaptationType> {
        let mut adaptations = vec![];

        if metrics.memory_pressure < 0.60 {
            adaptations.push(AdaptationType::ResumeFeatures);
        }

        if metrics.cpu_load < 0.50 && metrics.memory_pressure < 0.70 {
            adaptations.push(AdaptationType::ResumeFeatures);
        }

        adaptations
    }

    /// Get adaptation history
    pub fn history(&self) -> Vec<AdaptationEvent> {
        self.adaptation_history.read().clone()
    }

    /// Get adaptation summary
    pub fn adaptation_summary(&self) -> AdaptationSummary {
        let history = self.adaptation_history.read();

        let mut type_counts = std::collections::HashMap::new();
        let mut successful = 0u32;

        for event in history.iter() {
            *type_counts.entry(event.adaptation_type).or_insert(0u32) += 1;
            if event.applied {
                successful += 1;
            }
        }

        AdaptationSummary {
            total_events: history.len(),
            successful_adaptations: successful as usize,
            adaptation_distribution: type_counts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationSummary {
    pub total_events: usize,
    pub successful_adaptations: usize,
    pub adaptation_distribution: std::collections::HashMap<AdaptationType, u32>,
}

impl Default for RuntimeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_adapter_creation() {
        let adapter = RuntimeAdapter::new();
        assert_eq!(adapter.history().len(), 0);
    }

    #[test]
    fn test_memory_pressure_adaptations() {
        let adapter = RuntimeAdapter::new();
        let mut metrics = ResourceMetrics::default();

        metrics.memory_pressure = 0.85;
        let adaptations = adapter.analyze_metrics(&metrics, MemoryTrend::Stable);

        assert!(adaptations.len() > 0);
        assert!(adaptations.contains(&AdaptationType::ReduceCacheSize));
    }

    #[test]
    fn test_critical_state_detection() {
        let adapter = RuntimeAdapter::new();
        let mut metrics = ResourceMetrics::default();

        metrics.memory_pressure = 0.96;
        assert!(adapter.is_critical_state(&metrics));

        metrics.memory_pressure = 0.50;
        assert!(!adapter.is_critical_state(&metrics));
    }

    #[test]
    fn test_adaptation_logging() {
        let adapter = RuntimeAdapter::new();

        adapter.log_adaptation(
            AdaptationType::ReduceAnimations,
            "Memory pressure".to_string(),
            true,
        );

        let history = adapter.history();
        assert_eq!(history.len(), 1);
        assert!(history[0].applied);
    }

    #[test]
    fn test_recovery_adaptations() {
        let adapter = RuntimeAdapter::new();
        let mut metrics = ResourceMetrics::default();

        metrics.memory_pressure = 0.50;
        metrics.cpu_load = 0.40;

        let recovery = adapter.recovery_adaptations(&metrics);
        assert!(recovery.contains(&AdaptationType::ResumeFeatures));
    }

    #[test]
    fn test_adaptation_priority() {
        let adapter = RuntimeAdapter::new();

        let p1 = adapter.adaptation_priority(&AdaptationType::FreezeInactiveTabs);
        let p2 = adapter.adaptation_priority(&AdaptationType::ReduceAnimations);

        assert!(p1 < p2); // FreezeInactiveTabs should have higher priority
    }

    #[test]
    fn test_summary() {
        let adapter = RuntimeAdapter::new();

        adapter.log_adaptation(AdaptationType::ReduceAnimations, "test1".to_string(), true);
        adapter.log_adaptation(AdaptationType::ReduceGpuLoad, "test2".to_string(), false);

        let summary = adapter.adaptation_summary();
        assert_eq!(summary.total_events, 2);
        assert_eq!(summary.successful_adaptations, 1);
    }
}
