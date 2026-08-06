use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

use crate::intelligence::device_detection::{DeviceCapabilities, DeviceTier};
use crate::intelligence::resource_monitor::ResourceMonitor;

/// Adaptive operating profiles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdaptiveProfile {
    UltraCapability,
    HighCapability,
    Standard,
    LowMemory,
    PowerSaver,
}

impl std::fmt::Display for AdaptiveProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UltraCapability => write!(f, "Ultra Capability"),
            Self::HighCapability => write!(f, "High Capability"),
            Self::Standard => write!(f, "Standard"),
            Self::LowMemory => write!(f, "Low Memory"),
            Self::PowerSaver => write!(f, "Power Saver"),
        }
    }
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: AdaptiveProfile,
    pub ai_enabled: bool,
    pub local_models_enabled: bool,
    pub vision_enabled: bool,
    pub background_indexing: bool,
    pub max_parallel_tasks: usize,
    pub cache_size_mb: u32,
    pub memory_reserve_mb: u32,
    pub gpu_acceleration: bool,
    pub annotation_features: bool,
    pub multi_agent_support: bool,
}

pub struct ProfileManager;

impl ProfileManager {
    pub fn new() -> Self {
        Self
    }

    /// Select profile based on device capabilities and resources
    pub fn select_profile(
        device_caps: &DeviceCapabilities,
        resource_monitor: &ResourceMonitor,
    ) -> Result<AdaptiveProfile> {
        let tier = device_caps.device_tier();
        let metrics = resource_monitor.snapshot();
        let memory_pressure = metrics.memory_pressure;
        let is_low_battery = metrics.battery_level < 20 && !metrics.is_plugged_in;

        debug!(
            "Selecting profile for tier: {:?}, memory_pressure: {:.2}",
            tier, memory_pressure
        );

        // If under severe resource pressure, choose conservative profile
        if memory_pressure > 0.85 || is_low_battery {
            return Ok(AdaptiveProfile::PowerSaver);
        }

        // If moderate pressure, be cautious
        if memory_pressure > 0.70 {
            return Ok(match tier {
                DeviceTier::UltraCapability => AdaptiveProfile::HighCapability,
                DeviceTier::HighCapability => AdaptiveProfile::Standard,
                DeviceTier::Standard => AdaptiveProfile::LowMemory,
                _ => AdaptiveProfile::PowerSaver,
            });
        }

        // Normal operation: match to device tier
        Ok(match tier {
            DeviceTier::UltraCapability => AdaptiveProfile::UltraCapability,
            DeviceTier::HighCapability => AdaptiveProfile::HighCapability,
            DeviceTier::Standard => AdaptiveProfile::Standard,
            DeviceTier::LowMemory => AdaptiveProfile::LowMemory,
            DeviceTier::Constrained => AdaptiveProfile::PowerSaver,
        })
    }

    /// Get configuration for profile
    pub fn config_for(&self, profile: &AdaptiveProfile) -> ProfileConfig {
        match profile {
            AdaptiveProfile::UltraCapability => ProfileConfig {
                name: AdaptiveProfile::UltraCapability,
                ai_enabled: true,
                local_models_enabled: true,
                vision_enabled: true,
                background_indexing: true,
                max_parallel_tasks: 16,
                cache_size_mb: 2048,
                memory_reserve_mb: 2048,
                gpu_acceleration: true,
                annotation_features: true,
                multi_agent_support: true,
            },
            AdaptiveProfile::HighCapability => ProfileConfig {
                name: AdaptiveProfile::HighCapability,
                ai_enabled: true,
                local_models_enabled: true,
                vision_enabled: false,
                background_indexing: true,
                max_parallel_tasks: 8,
                cache_size_mb: 1024,
                memory_reserve_mb: 1024,
                gpu_acceleration: true,
                annotation_features: true,
                multi_agent_support: false,
            },
            AdaptiveProfile::Standard => ProfileConfig {
                name: AdaptiveProfile::Standard,
                ai_enabled: true,
                local_models_enabled: false,
                vision_enabled: false,
                background_indexing: false,
                max_parallel_tasks: 4,
                cache_size_mb: 512,
                memory_reserve_mb: 512,
                gpu_acceleration: false,
                annotation_features: true,
                multi_agent_support: false,
            },
            AdaptiveProfile::LowMemory => ProfileConfig {
                name: AdaptiveProfile::LowMemory,
                ai_enabled: false,
                local_models_enabled: false,
                vision_enabled: false,
                background_indexing: false,
                max_parallel_tasks: 2,
                cache_size_mb: 128,
                memory_reserve_mb: 256,
                gpu_acceleration: false,
                annotation_features: false,
                multi_agent_support: false,
            },
            AdaptiveProfile::PowerSaver => ProfileConfig {
                name: AdaptiveProfile::PowerSaver,
                ai_enabled: false,
                local_models_enabled: false,
                vision_enabled: false,
                background_indexing: false,
                max_parallel_tasks: 1,
                cache_size_mb: 64,
                memory_reserve_mb: 128,
                gpu_acceleration: false,
                annotation_features: false,
                multi_agent_support: false,
            },
        }
    }

    /// Check if feature is enabled in profile
    pub fn feature_enabled(&self, profile: &AdaptiveProfile, feature: &str) -> bool {
        let config = self.config_for(profile);

        match feature {
            "ai_assistant" => config.ai_enabled,
            "local_models" => config.local_models_enabled,
            "vision" => config.vision_enabled,
            "indexing" => config.background_indexing,
            "gpu" => config.gpu_acceleration,
            "annotations" => config.annotation_features,
            "agents" => config.multi_agent_support,
            _ => false,
        }
    }

    /// Get memory budget for profile
    pub fn memory_budget_mb(&self, profile: &AdaptiveProfile) -> u32 {
        let config = self.config_for(profile);
        config.cache_size_mb
    }

    /// Describe profile for user
    pub fn describe(&self, profile: &AdaptiveProfile) -> String {
        let config = self.config_for(profile);

        let mut features = vec![];
        if config.ai_enabled {
            features.push("AI Assistant");
        }
        if config.local_models_enabled {
            features.push("Local Models");
        }
        if config.vision_enabled {
            features.push("Vision Processing");
        }
        if config.background_indexing {
            features.push("Background Indexing");
        }
        if config.gpu_acceleration {
            features.push("GPU Acceleration");
        }
        if config.annotation_features {
            features.push("Annotations");
        }

        format!(
            "{} Profile\nEnabled: {}\nMemory: {}MB\nParallel Tasks: {}",
            profile,
            features.join(", "),
            config.cache_size_mb,
            config.max_parallel_tasks
        )
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_manager_creation() {
        let manager = ProfileManager::new();
        let profile = AdaptiveProfile::Standard;
        let config = manager.config_for(&profile);

        assert_eq!(config.name, profile);
    }

    #[test]
    fn test_ultra_capability_config() {
        let manager = ProfileManager::new();
        let config = manager.config_for(&AdaptiveProfile::UltraCapability);

        assert!(config.ai_enabled);
        assert!(config.local_models_enabled);
        assert!(config.vision_enabled);
        assert!(config.gpu_acceleration);
    }

    #[test]
    fn test_power_saver_config() {
        let manager = ProfileManager::new();
        let config = manager.config_for(&AdaptiveProfile::PowerSaver);

        assert!(!config.ai_enabled);
        assert!(!config.gpu_acceleration);
        assert_eq!(config.max_parallel_tasks, 1);
    }

    #[test]
    fn test_feature_enabled_checks() {
        let manager = ProfileManager::new();
        let ultra = AdaptiveProfile::UltraCapability;
        let power_saver = AdaptiveProfile::PowerSaver;

        assert!(manager.feature_enabled(&ultra, "ai_assistant"));
        assert!(!manager.feature_enabled(&power_saver, "ai_assistant"));
    }

    #[test]
    fn test_memory_budget() {
        let manager = ProfileManager::new();

        let ultra_budget = manager.memory_budget_mb(&AdaptiveProfile::UltraCapability);
        let ps_budget = manager.memory_budget_mb(&AdaptiveProfile::PowerSaver);

        assert!(ultra_budget > ps_budget);
    }

    #[test]
    fn test_profile_description() {
        let manager = ProfileManager::new();
        let desc = manager.describe(&AdaptiveProfile::Standard);

        assert!(desc.contains("Standard"));
        assert!(desc.contains("Memory"));
    }
}
