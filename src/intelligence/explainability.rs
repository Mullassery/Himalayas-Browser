use serde::{Deserialize, Serialize};

use crate::intelligence::device_detection::DeviceCapabilities;
use crate::intelligence::profile_manager::AdaptiveProfile;
use crate::intelligence::resource_monitor::ResourceMetrics;

/// User-facing explanations of intelligence decisions
pub struct ProfileExplainer;

impl ProfileExplainer {
    pub fn new() -> Self {
        Self
    }

    /// Explain device analysis to user
    pub fn explain_device(&self, capabilities: &DeviceCapabilities) -> String {
        let tier = capabilities.device_tier();

        format!(
            "Device Analysis Complete\n\n\
            Hardware Detected:\n\
            ✓ CPU: {} cores ({:?} performance)\n\
            ✓ RAM: {:.1}GB total ({:.1}GB available)\n\
            ✓ GPU: {}\n\
            ✓ Storage: {:.1}GB SSD ({}MB/s read)\n\
            ✓ Battery: {}\n\n\
            Device Classification: {:?}\n\n\
            This configuration enables the following capabilities:\n\
            {}",
            capabilities.cpu.cores,
            capabilities.cpu.performance_class,
            capabilities.memory.total_ram_gb,
            capabilities.memory.available_ram_gb,
            if capabilities.gpu.has_gpu {
                format!("{:?}", capabilities.gpu.gpu_type)
            } else {
                "No GPU".to_string()
            },
            capabilities.storage.total_gb,
            capabilities.storage.sequential_read_mbps,
            if capabilities.battery.has_battery {
                format!(
                    "{}% ({})",
                    capabilities.battery.battery_level_percent,
                    if capabilities.battery.is_plugged_in {
                        "plugged in"
                    } else {
                        "on battery"
                    }
                )
            } else {
                "Desktop (no battery)".to_string()
            },
            tier,
            self.capabilities_for_tier(&tier)
        )
    }

    /// Explain current profile configuration
    pub fn explain_profile(
        &self,
        profile: &AdaptiveProfile,
        device: &DeviceCapabilities,
        metrics: ResourceMetrics,
    ) -> String {
        format!(
            "Active Configuration: {} Profile\n\n\
            Current Status:\n\
            • Memory usage: {:.0}%\n\
            • CPU load: {:.0}%\n\
            • Battery: {}%\n\
            • Thermal: {}\n\n\
            Enabled Features:\n\
            {}\n\n\
            Optimizations Active:\n\
            {}\n\n\
            Reason:\n\
            {}",
            profile,
            metrics.memory_pressure * 100.0,
            metrics.cpu_load * 100.0,
            metrics.battery_level,
            if metrics.thermal_throttling {
                "🔥 Throttling"
            } else {
                "✓ Normal"
            },
            self.features_explanation(profile),
            self.optimizations_explanation(profile),
            self.reason_explanation(profile, &metrics)
        )
    }

    fn capabilities_for_tier(&self, tier: &crate::intelligence::device_detection::DeviceTier) -> String {
        use crate::intelligence::device_detection::DeviceTier;

        match tier {
            DeviceTier::UltraCapability => {
                "✓ Advanced AI assistant with local models\n\
                 ✓ Vision understanding\n\
                 ✓ Real-time semantic indexing\n\
                 ✓ Multi-agent workflows\n\
                 ✓ GPU-accelerated rendering\n\
                 ✓ Persistent knowledge graph"
                    .to_string()
            }
            DeviceTier::HighCapability => {
                "✓ AI assistant with smart caching\n\
                 ✓ Lightweight semantic search\n\
                 ✓ Annotation support\n\
                 ✓ GPU acceleration\n\
                 ✓ Background processing"
                    .to_string()
            }
            DeviceTier::Standard => {
                "✓ AI assistant\n\
                 ✓ Tab optimization\n\
                 ✓ Annotation support\n\
                 ✓ Smart caching"
                    .to_string()
            }
            DeviceTier::LowMemory => {
                "✓ Essential AI functions\n\
                 ✓ Lightweight rendering\n\
                 ✓ Efficient tab management"
                    .to_string()
            }
            DeviceTier::Constrained => {
                "✓ Core browsing\n\
                 ✓ Stable performance"
                    .to_string()
            }
        }
    }

    fn features_explanation(&self, profile: &AdaptiveProfile) -> String {
        match profile {
            AdaptiveProfile::UltraCapability => {
                "✓ AI Assistant\n  ✓ Local Models\n  ✓ Vision Processing\n\
                 ✓ Semantic Indexing\n  ✓ Knowledge Graph\n  ✓ Multi-Agent Support"
                    .to_string()
            }
            AdaptiveProfile::HighCapability => {
                "✓ AI Assistant\n  ✓ Local Models\n  ✓ Semantic Indexing\n\
                 ✓ GPU Acceleration\n  ✓ Annotations"
                    .to_string()
            }
            AdaptiveProfile::Standard => {
                "✓ AI Assistant\n  ✓ Smart Caching\n  ✓ Annotations\n\
                 ✓ Tab Optimization"
                    .to_string()
            }
            AdaptiveProfile::LowMemory => {
                "✓ Lightweight Rendering\n  ✓ Essential AI\n\
                 ✓ Aggressive Memory Management"
                    .to_string()
            }
            AdaptiveProfile::PowerSaver => {
                "✓ Core Features Only\n  ✓ Battery Preservation\n\
                 ✓ Minimal Background Activity"
                    .to_string()
            }
        }
    }

    fn optimizations_explanation(&self, profile: &AdaptiveProfile) -> String {
        match profile {
            AdaptiveProfile::UltraCapability => {
                "• Full parallel processing\n\
                 • 2GB cache\n\
                 • Predictive preloading\n\
                 • Continuous indexing"
                    .to_string()
            }
            AdaptiveProfile::HighCapability => {
                "• Smart parallel processing\n\
                 • 1GB cache\n\
                 • Selective indexing"
                    .to_string()
            }
            AdaptiveProfile::Standard => {
                "• Balanced parallelism\n\
                 • 512MB cache\n\
                 • On-demand processing"
                    .to_string()
            }
            AdaptiveProfile::LowMemory => {
                "• Serial processing\n\
                 • 128MB cache\n\
                 • Aggressive cleanup"
                    .to_string()
            }
            AdaptiveProfile::PowerSaver => {
                "• Minimal overhead\n\
                 • 64MB cache\n\
                 • Hibernation mode"
                    .to_string()
            }
        }
    }

    fn reason_explanation(&self, profile: &AdaptiveProfile, metrics: &ResourceMetrics) -> String {
        if metrics.memory_pressure > 0.80 {
            "Reduced profile due to high memory pressure".to_string()
        } else if metrics.battery_level < 20 && !metrics.is_plugged_in {
            "Power saver mode activated to extend battery life".to_string()
        } else if metrics.thermal_throttling {
            "Thermal management: reducing heat generation".to_string()
        } else {
            format!(
                "Optimized for {} - balancing performance and efficiency",
                profile
            )
        }
    }

    /// Create user-friendly summary
    pub fn summary(&self, profile: &AdaptiveProfile) -> String {
        match profile {
            AdaptiveProfile::UltraCapability => {
                "Maximum Performance\nAll features enabled for ultimate capability".to_string()
            }
            AdaptiveProfile::HighCapability => {
                "High Performance\nAdvanced features with efficient resource use".to_string()
            }
            AdaptiveProfile::Standard => {
                "Balanced Mode\nOptimal for most use cases".to_string()
            }
            AdaptiveProfile::LowMemory => {
                "Efficient Mode\nMaximum compatibility on limited resources".to_string()
            }
            AdaptiveProfile::PowerSaver => {
                "Battery Saver\nMinimizing power consumption".to_string()
            }
        }
    }
}

impl Default for ProfileExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explainer_creation() {
        let explainer = ProfileExplainer::new();
        assert!(true); // Just test it doesn't panic
    }

    #[test]
    fn test_device_explanation() {
        let explainer = ProfileExplainer::new();
        let device = DeviceCapabilities::detect().unwrap();

        let explanation = explainer.explain_device(&device);
        assert!(explanation.contains("Device Analysis Complete"));
        assert!(explanation.contains("CPU"));
        assert!(explanation.contains("RAM"));
    }

    #[test]
    fn test_profile_explanation() {
        let explainer = ProfileExplainer::new();
        let device = DeviceCapabilities::detect().unwrap();
        let metrics = ResourceMetrics::default();

        let explanation = explainer.explain_profile(&AdaptiveProfile::Standard, &device, metrics);
        assert!(explanation.contains("Standard"));
        assert!(explanation.contains("Status"));
    }

    #[test]
    fn test_summary_for_profiles() {
        let explainer = ProfileExplainer::new();

        for profile in &[
            AdaptiveProfile::UltraCapability,
            AdaptiveProfile::HighCapability,
            AdaptiveProfile::Standard,
        ] {
            let summary = explainer.summary(profile);
            assert!(!summary.is_empty());
        }
    }
}
