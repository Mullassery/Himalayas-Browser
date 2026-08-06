pub mod device_detection;
pub mod resource_monitor;
pub mod workload_analyzer;
pub mod profile_manager;
pub mod feature_loader;
pub mod runtime_adapter;
pub mod explainability;

pub use device_detection::DeviceCapabilities;
pub use resource_monitor::ResourceMonitor;
pub use workload_analyzer::WorkloadAnalyzer;
pub use profile_manager::{ProfileManager, AdaptiveProfile};
pub use feature_loader::FeatureLoader;
pub use runtime_adapter::RuntimeAdapter;
pub use explainability::ProfileExplainer;

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug};
use serde::{Deserialize, Serialize};

/// Intelligent Resource Manager
/// Orchestrates all adaptation decisions
pub struct IntelligenceEngine {
    device_caps: Arc<DeviceCapabilities>,
    resource_monitor: Arc<ResourceMonitor>,
    workload_analyzer: Arc<WorkloadAnalyzer>,
    profile_manager: Arc<ProfileManager>,
    feature_loader: Arc<FeatureLoader>,
    runtime_adapter: Arc<RuntimeAdapter>,
    explainer: Arc<ProfileExplainer>,
    current_profile: Arc<RwLock<AdaptiveProfile>>,
}

impl IntelligenceEngine {
    pub async fn new() -> Result<Self> {
        info!("🧠 Initializing Intelligent Resource Manager");

        let device_caps = Arc::new(DeviceCapabilities::detect()?);
        let resource_monitor = Arc::new(ResourceMonitor::new()?);
        let workload_analyzer = Arc::new(WorkloadAnalyzer::new());
        let profile_manager = Arc::new(ProfileManager::new());
        let feature_loader = Arc::new(FeatureLoader::new());
        let runtime_adapter = Arc::new(RuntimeAdapter::new());
        let explainer = Arc::new(ProfileExplainer::new());

        // Initial profile selection
        let initial_profile = ProfileManager::select_profile(
            device_caps.as_ref(),
            resource_monitor.as_ref(),
        )?;

        info!("Initial profile selected: {:?}", initial_profile);

        Ok(Self {
            device_caps,
            resource_monitor,
            workload_analyzer,
            profile_manager,
            feature_loader,
            runtime_adapter,
            explainer,
            current_profile: Arc::new(RwLock::new(initial_profile)),
        })
    }

    /// Analyze device capabilities at startup
    pub fn device_analysis(&self) -> String {
        self.explainer.explain_device(&self.device_caps)
    }

    /// Get current adaptive profile
    pub fn current_profile(&self) -> AdaptiveProfile {
        self.current_profile.read().clone()
    }

    /// Evaluate workload and adjust profile
    pub async fn evaluate_workload(&self, workload: WorkloadContext) -> Result<()> {
        debug!("Evaluating workload: {:?}", workload.workload_type);

        self.workload_analyzer.record_activity(workload.clone());

        // Check resource pressure
        let metrics = self.resource_monitor.snapshot();
        if metrics.memory_pressure > 0.85 || metrics.cpu_load > 0.90 {
            self.adapt_for_pressure().await?;
        }

        Ok(())
    }

    /// Adapt profile based on current resource pressure
    async fn adapt_for_pressure(&self) -> Result<()> {
        debug!("Resource pressure detected, adapting profile");

        let new_profile = ProfileManager::select_profile(
            self.device_caps.as_ref(),
            self.resource_monitor.as_ref(),
        )?;

        let mut current = self.current_profile.write();
        *current = new_profile;

        Ok(())
    }

    /// Explain current profile configuration
    pub fn explain_profile(&self) -> String {
        let profile = self.current_profile.read();
        self.explainer.explain_profile(
            &profile,
            self.device_caps.as_ref(),
            self.resource_monitor.snapshot(),
        )
    }

    /// Get feature availability for current profile
    pub fn available_features(&self) -> Vec<String> {
        let profile = self.current_profile.read();
        self.feature_loader.features_for_profile(&profile)
    }

    /// Check if feature should be loaded
    pub fn should_load_feature(&self, feature: &str) -> bool {
        let profile = self.current_profile.read();
        self.feature_loader.should_load(&profile, feature)
    }
}

#[derive(Debug, Clone)]
pub struct WorkloadContext {
    pub workload_type: WorkloadType,
    pub num_tabs: usize,
    pub gpu_required: bool,
    pub ai_required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    Reading,
    VideoStreaming,
    Gaming,
    Development,
    Research,
    AiAssisted,
    Enterprise,
    Idle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intelligence_engine_creation() {
        let engine = IntelligenceEngine::new().await.unwrap();
        assert!(engine.device_analysis().len() > 0);
    }

    #[tokio::test]
    async fn test_current_profile() {
        let engine = IntelligenceEngine::new().await.unwrap();
        let profile = engine.current_profile();
        // Verify we got a valid profile
        assert_ne!(profile, AdaptiveProfile::PowerSaver);
    }

    #[tokio::test]
    async fn test_feature_availability() {
        let engine = IntelligenceEngine::new().await.unwrap();
        let features = engine.available_features();
        assert!(features.len() > 0);
    }
}
