use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::intelligence::profile_manager::AdaptiveProfile;

/// Dynamic feature loader
pub struct FeatureLoader {
    feature_requirements: HashMap<String, FeatureRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureRequirement {
    pub name: String,
    pub min_ram_mb: u32,
    pub min_profile: FeatureMinProfile,
    pub conflicts_with: Vec<String>,
    pub load_time_ms: u32,
    pub memory_overhead_mb: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeatureMinProfile {
    Any,
    Standard,
    HighCapability,
    UltraCapability,
}

impl FeatureLoader {
    pub fn new() -> Self {
        debug!("Initializing Feature Loader");

        let mut features = HashMap::new();

        // Define feature requirements
        features.insert(
            "ai_assistant".to_string(),
            FeatureRequirement {
                name: "AI Assistant".to_string(),
                min_ram_mb: 2048,
                min_profile: FeatureMinProfile::Standard,
                conflicts_with: vec![],
                load_time_ms: 500,
                memory_overhead_mb: 512,
            },
        );

        features.insert(
            "local_models".to_string(),
            FeatureRequirement {
                name: "Local LLM Models".to_string(),
                min_ram_mb: 8192,
                min_profile: FeatureMinProfile::HighCapability,
                conflicts_with: vec![],
                load_time_ms: 2000,
                memory_overhead_mb: 4096,
            },
        );

        features.insert(
            "vision_processing".to_string(),
            FeatureRequirement {
                name: "Vision Processing".to_string(),
                min_ram_mb: 6144,
                min_profile: FeatureMinProfile::HighCapability,
                conflicts_with: vec![],
                load_time_ms: 800,
                memory_overhead_mb: 2048,
            },
        );

        features.insert(
            "semantic_indexing".to_string(),
            FeatureRequirement {
                name: "Semantic Indexing".to_string(),
                min_ram_mb: 4096,
                min_profile: FeatureMinProfile::HighCapability,
                conflicts_with: vec![],
                load_time_ms: 1500,
                memory_overhead_mb: 1024,
            },
        );

        features.insert(
            "knowledge_graph".to_string(),
            FeatureRequirement {
                name: "Knowledge Graph".to_string(),
                min_ram_mb: 8192,
                min_profile: FeatureMinProfile::UltraCapability,
                conflicts_with: vec![],
                load_time_ms: 3000,
                memory_overhead_mb: 2048,
            },
        );

        features.insert(
            "multi_agent".to_string(),
            FeatureRequirement {
                name: "Multi-Agent Runtime".to_string(),
                min_ram_mb: 6144,
                min_profile: FeatureMinProfile::HighCapability,
                conflicts_with: vec![],
                load_time_ms: 1000,
                memory_overhead_mb: 1536,
            },
        );

        features.insert(
            "gpu_acceleration".to_string(),
            FeatureRequirement {
                name: "GPU Acceleration".to_string(),
                min_ram_mb: 2048,
                min_profile: FeatureMinProfile::Standard,
                conflicts_with: vec![],
                load_time_ms: 1500,
                memory_overhead_mb: 512,
            },
        );

        features.insert(
            "advanced_annotations".to_string(),
            FeatureRequirement {
                name: "Advanced Annotations".to_string(),
                min_ram_mb: 1024,
                min_profile: FeatureMinProfile::Standard,
                conflicts_with: vec![],
                load_time_ms: 200,
                memory_overhead_mb: 128,
            },
        );

        features.insert(
            "predictive_loading".to_string(),
            FeatureRequirement {
                name: "Predictive Preloading".to_string(),
                min_ram_mb: 4096,
                min_profile: FeatureMinProfile::HighCapability,
                conflicts_with: vec![],
                load_time_ms: 300,
                memory_overhead_mb: 256,
            },
        );

        features.insert(
            "real_time_translation".to_string(),
            FeatureRequirement {
                name: "Real-Time Translation".to_string(),
                min_ram_mb: 2048,
                min_profile: FeatureMinProfile::Standard,
                conflicts_with: vec![],
                load_time_ms: 600,
                memory_overhead_mb: 256,
            },
        );

        Self {
            feature_requirements: features,
        }
    }

    /// Check if feature should be loaded for profile
    pub fn should_load(&self, profile: &AdaptiveProfile, feature: &str) -> bool {
        if let Some(req) = self.feature_requirements.get(feature) {
            self.meets_profile_requirement(profile, &req.min_profile)
        } else {
            false
        }
    }

    /// Get all features available for profile
    pub fn features_for_profile(&self, profile: &AdaptiveProfile) -> Vec<String> {
        self.feature_requirements
            .iter()
            .filter(|(_, req)| self.meets_profile_requirement(profile, &req.min_profile))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get feature requirement
    pub fn get_requirement(&self, feature: &str) -> Option<FeatureRequirement> {
        self.feature_requirements.get(feature).cloned()
    }

    /// Calculate total load time for features
    pub fn total_load_time_ms(&self, features: &[&str]) -> u32 {
        features
            .iter()
            .filter_map(|f| self.feature_requirements.get(*f))
            .map(|r| r.load_time_ms)
            .sum()
    }

    /// Calculate total memory overhead for features
    pub fn total_memory_overhead_mb(&self, features: &[&str]) -> u32 {
        features
            .iter()
            .filter_map(|f| self.feature_requirements.get(*f))
            .map(|r| r.memory_overhead_mb)
            .sum()
    }

    /// Suggest features to unload if memory pressure
    pub fn suggest_unload(&self, profile: &AdaptiveProfile, pressure: f32) -> Vec<String> {
        if pressure < 0.70 {
            return vec![];
        }

        // Unload heaviest features first
        let mut features: Vec<_> = self
            .feature_requirements
            .iter()
            .filter(|(_, req)| self.meets_profile_requirement(profile, &req.min_profile))
            .collect();

        features.sort_by_key(|(_, req)| std::cmp::Reverse(req.memory_overhead_mb));

        features
            .into_iter()
            .take((pressure - 0.70) as usize / 10 + 1)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn meets_profile_requirement(&self, profile: &AdaptiveProfile, min_req: &FeatureMinProfile) -> bool {
        match (profile, min_req) {
            (_, FeatureMinProfile::Any) => true,
            (AdaptiveProfile::UltraCapability, _) => true,
            (AdaptiveProfile::HighCapability, FeatureMinProfile::HighCapability) => true,
            (AdaptiveProfile::HighCapability, FeatureMinProfile::UltraCapability) => false,
            (AdaptiveProfile::Standard, FeatureMinProfile::Standard) => true,
            (AdaptiveProfile::Standard, _) => false,
            _ => false,
        }
    }
}

impl Default for FeatureLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_loader_creation() {
        let loader = FeatureLoader::new();
        assert!(loader.get_requirement("ai_assistant").is_some());
    }

    #[test]
    fn test_features_for_ultra_profile() {
        let loader = FeatureLoader::new();
        let features = loader.features_for_profile(&AdaptiveProfile::UltraCapability);

        assert!(features.contains(&"ai_assistant".to_string()));
        assert!(features.contains(&"local_models".to_string()));
        assert!(features.contains(&"knowledge_graph".to_string()));
    }

    #[test]
    fn test_features_for_low_memory() {
        let loader = FeatureLoader::new();
        let features = loader.features_for_profile(&AdaptiveProfile::LowMemory);

        // Low memory profile should have minimal features
        assert!(features.is_empty() || features.len() < 3);
    }

    #[test]
    fn test_should_load() {
        let loader = FeatureLoader::new();

        assert!(loader.should_load(&AdaptiveProfile::UltraCapability, "ai_assistant"));
        assert!(!loader.should_load(&AdaptiveProfile::PowerSaver, "ai_assistant"));
    }

    #[test]
    fn test_load_time_calculation() {
        let loader = FeatureLoader::new();
        let time = loader.total_load_time_ms(&["ai_assistant", "vision_processing"]);

        assert!(time > 0);
    }

    #[test]
    fn test_memory_overhead_calculation() {
        let loader = FeatureLoader::new();
        let overhead = loader.total_memory_overhead_mb(&["ai_assistant", "local_models"]);

        assert!(overhead > 0);
    }

    #[test]
    fn test_unload_suggestions() {
        let loader = FeatureLoader::new();
        let unload = loader.suggest_unload(&AdaptiveProfile::HighCapability, 0.85);

        // Under high pressure, should suggest something to unload
        assert!(unload.len() > 0);
    }
}
