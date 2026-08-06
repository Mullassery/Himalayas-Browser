use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use crate::intelligence::{WorkloadContext, WorkloadType};

/// Analyzes user workload patterns
pub struct WorkloadAnalyzer {
    recent_activities: Arc<RwLock<VecDeque<WorkloadEntry>>>,
    max_entries: usize,
    detected_workload: Arc<RwLock<DetectedWorkload>>,
}

#[derive(Debug, Clone)]
struct WorkloadEntry {
    timestamp_ms: u64,
    workload_type: WorkloadType,
    duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DetectedWorkload {
    pub primary_workload: WorkloadType,
    pub intensity: f32,           // 0.0-1.0
    pub is_interactive: bool,
    pub estimated_cpu_time_ms: u32,
    pub gpu_utilization: f32,     // 0.0-1.0
}

impl WorkloadAnalyzer {
    pub fn new() -> Self {
        debug!("Initializing Workload Analyzer");

        Self {
            recent_activities: Arc::new(RwLock::new(VecDeque::new())),
            max_entries: 50,
            detected_workload: Arc::new(RwLock::new(DetectedWorkload {
                primary_workload: WorkloadType::Idle,
                intensity: 0.0,
                is_interactive: false,
                estimated_cpu_time_ms: 0,
                gpu_utilization: 0.0,
            })),
        }
    }

    /// Record user activity
    pub fn record_activity(&self, context: WorkloadContext) {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let entry = WorkloadEntry {
            timestamp_ms,
            workload_type: context.workload_type,
            duration_ms: 0,
        };

        let mut activities = self.recent_activities.write();
        if activities.len() >= self.max_entries {
            activities.pop_front();
        }
        activities.push_back(entry);

        // Re-analyze workload
        drop(activities); // Release lock
        self.analyze_pattern();
    }

    /// Analyze activity patterns
    fn analyze_pattern(&self) {
        let activities = self.recent_activities.read();

        if activities.is_empty() {
            return;
        }

        // Count workload types
        let mut counts = std::collections::HashMap::new();
        for entry in activities.iter() {
            *counts.entry(entry.workload_type).or_insert(0) += 1;
        }

        // Determine primary workload
        let primary = counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&wt, _)| wt)
            .unwrap_or(WorkloadType::Idle);

        // Calculate intensity (tabs open, GPU usage)
        let num_recent = activities.len() as f32;
        let intensity = (num_recent / 20.0).clamp(0.0, 1.0);

        let detected = DetectedWorkload {
            primary_workload: primary,
            intensity,
            is_interactive: matches!(
                primary,
                WorkloadType::Gaming | WorkloadType::Development | WorkloadType::Enterprise
            ),
            estimated_cpu_time_ms: (intensity * 1000.0) as u32,
            gpu_utilization: if matches!(primary, WorkloadType::Gaming | WorkloadType::VideoStreaming) {
                intensity
            } else {
                0.0
            },
        };

        *self.detected_workload.write() = detected;
    }

    /// Get current detected workload
    pub fn current_workload(&self) -> DetectedWorkload {
        self.detected_workload.read().clone()
    }

    /// Check if workload requires GPU
    pub fn requires_gpu(&self) -> bool {
        let workload = self.detected_workload.read();
        workload.gpu_utilization > 0.2
    }

    /// Check if workload requires AI
    pub fn requires_ai(&self) -> bool {
        let workload = self.detected_workload.read();
        matches!(
            workload.primary_workload,
            WorkloadType::AiAssisted
        )
    }

    /// Predict next workload type
    pub fn predict_next_workload(&self) -> WorkloadType {
        let activities = self.recent_activities.read();

        if activities.len() < 2 {
            return WorkloadType::Idle;
        }

        // Simple prediction: if pattern changes, expect transition
        let last = activities.back().map(|e| e.workload_type).unwrap_or(WorkloadType::Idle);

        // In a real system, would use ML model
        last
    }

    /// Get activity history summary
    pub fn activity_summary(&self) -> ActivitySummary {
        let activities = self.recent_activities.read();
        let mut type_counts = std::collections::HashMap::new();

        for entry in activities.iter() {
            *type_counts.entry(entry.workload_type).or_insert(0) += 1;
        }

        ActivitySummary {
            total_activities: activities.len(),
            workload_distribution: type_counts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub total_activities: usize,
    pub workload_distribution: std::collections::HashMap<WorkloadType, usize>,
}

impl Default for WorkloadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::WorkloadType as CtxWorkloadType;

    #[test]
    fn test_workload_analyzer_creation() {
        let analyzer = WorkloadAnalyzer::new();
        let workload = analyzer.current_workload();
        assert_eq!(workload.primary_workload, WorkloadType::Idle);
    }

    #[test]
    fn test_activity_recording() {
        let analyzer = WorkloadAnalyzer::new();

        let context = WorkloadContext {
            workload_type: CtxWorkloadType::Reading,
            num_tabs: 5,
            gpu_required: false,
            ai_required: false,
        };

        analyzer.record_activity(context.clone());
        analyzer.record_activity(context);

        let summary = analyzer.activity_summary();
        assert!(summary.total_activities > 0);
    }

    #[test]
    fn test_workload_detection() {
        let analyzer = WorkloadAnalyzer::new();

        for _ in 0..10 {
            let context = WorkloadContext {
                workload_type: CtxWorkloadType::Gaming,
                num_tabs: 1,
                gpu_required: true,
                ai_required: false,
            };
            analyzer.record_activity(context);
        }

        let workload = analyzer.current_workload();
        assert_eq!(workload.primary_workload, WorkloadType::Gaming);
        assert!(workload.gpu_utilization > 0.0);
    }

    #[test]
    fn test_gpu_requirement_detection() {
        let analyzer = WorkloadAnalyzer::new();

        let context = WorkloadContext {
            workload_type: CtxWorkloadType::VideoStreaming,
            num_tabs: 2,
            gpu_required: true,
            ai_required: false,
        };

        // Record enough times to reach GPU utilization threshold (>0.2)
        for _ in 0..5 {
            analyzer.record_activity(context.clone());
        }
        assert!(analyzer.requires_gpu());
    }

    #[test]
    fn test_intensity_calculation() {
        let analyzer = WorkloadAnalyzer::new();

        // Record multiple activities
        for _ in 0..15 {
            let context = WorkloadContext {
                workload_type: CtxWorkloadType::Development,
                num_tabs: 8,
                gpu_required: false,
                ai_required: true,
            };
            analyzer.record_activity(context);
        }

        let workload = analyzer.current_workload();
        assert!(workload.intensity > 0.5);
    }
}
