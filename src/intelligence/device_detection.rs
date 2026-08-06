use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Device hardware capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceCapabilities {
    pub cpu: CpuCapabilities,
    pub gpu: GpuCapabilities,
    pub memory: MemoryCapabilities,
    pub storage: StorageCapabilities,
    pub network: NetworkCapabilities,
    pub battery: BatteryCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuCapabilities {
    pub architecture: String,     // x86_64, arm64, etc
    pub cores: usize,
    pub performance_class: PerformanceClass,
    pub supports_avx: bool,
    pub supports_neon: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PerformanceClass {
    UltraLow,   // < 1GHz
    Low,        // 1-2 GHz
    Medium,     // 2-3 GHz
    High,       // 3-4 GHz
    UltraHigh,  // > 4 GHz
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuCapabilities {
    pub has_gpu: bool,
    pub gpu_type: GpuType,
    pub vram_mb: u32,
    pub supports_vulkan: bool,
    pub supports_metal: bool,
    pub supports_dx12: bool,
    pub npu_available: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuType {
    None,
    Integrated,
    Discrete,
    Mobile,
    AppleSilicon,
    Npu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryCapabilities {
    pub total_ram_gb: f32,
    pub available_ram_gb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageCapabilities {
    pub total_gb: f32,
    pub available_gb: f32,
    pub is_ssd: bool,
    pub sequential_read_mbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkCapabilities {
    pub has_wifi: bool,
    pub has_5g: bool,
    pub has_ethernet: bool,
    pub estimated_bandwidth_mbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatteryCapabilities {
    pub has_battery: bool,
    pub is_plugged_in: bool,
    pub battery_level_percent: u8,
    pub thermal_throttling: bool,
}

impl DeviceCapabilities {
    pub fn detect() -> Result<Self> {
        debug!("Detecting device capabilities");

        let cpu = Self::detect_cpu();
        let gpu = Self::detect_gpu();
        let memory = Self::detect_memory();
        let storage = Self::detect_storage();
        let network = Self::detect_network();
        let battery = Self::detect_battery();

        info!("Device detected: {} cores, {}GB RAM", cpu.cores, memory.total_ram_gb);

        Ok(Self {
            cpu,
            gpu,
            memory,
            storage,
            network,
            battery,
        })
    }

    fn detect_cpu() -> CpuCapabilities {
        #[cfg(target_arch = "x86_64")]
        let (arch, supports_avx, supports_neon) = ("x86_64".to_string(), true, false);
        #[cfg(target_arch = "aarch64")]
        let (arch, supports_avx, supports_neon) = ("arm64".to_string(), false, true);
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let (arch, supports_avx, supports_neon) = ("unknown".to_string(), false, false);

        let cores = num_cpus::get();
        let performance_class = Self::estimate_cpu_performance();

        CpuCapabilities {
            architecture: arch,
            cores,
            performance_class,
            supports_avx,
            supports_neon,
        }
    }

    fn estimate_cpu_performance() -> PerformanceClass {
        // Simplified: assume modern CPUs are high-performance
        #[cfg(target_os = "macos")]
        {
            PerformanceClass::UltraHigh
        }
        #[cfg(not(target_os = "macos"))]
        {
            if num_cpus::get() >= 8 {
                PerformanceClass::High
            } else {
                PerformanceClass::Medium
            }
        }
    }

    fn detect_gpu() -> GpuCapabilities {
        #[cfg(target_os = "macos")]
        let gpu_type = GpuType::AppleSilicon;
        #[cfg(target_os = "windows")]
        let gpu_type = GpuType::Discrete;
        #[cfg(target_os = "linux")]
        let gpu_type = GpuType::Integrated;
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let gpu_type = GpuType::None;

        GpuCapabilities {
            has_gpu: gpu_type != GpuType::None,
            gpu_type,
            vram_mb: 2048,
            supports_vulkan: true,
            supports_metal: cfg!(target_os = "macos"),
            supports_dx12: cfg!(target_os = "windows"),
            npu_available: cfg!(target_os = "macos"),
        }
    }

    fn detect_memory() -> MemoryCapabilities {
        // Use platform-specific memory detection
        #[cfg(target_os = "macos")]
        {
            // macOS: use sysctl
            let output = std::process::Command::new("sysctl")
                .arg("-n")
                .arg("hw.memsize")
                .output();

            let total_bytes = if let Ok(out) = output {
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(8_000_000_000)
            } else {
                8_000_000_000
            };

            let total_ram_gb = total_bytes as f32 / 1_000_000_000.0;
            let available_ram_gb = (total_ram_gb * 0.8).clamp(0.5, total_ram_gb); // Estimate 80% available

            MemoryCapabilities {
                total_ram_gb,
                available_ram_gb,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Fallback: assume 8GB
            MemoryCapabilities {
                total_ram_gb: 8.0,
                available_ram_gb: 6.4,
            }
        }
    }

    fn detect_storage() -> StorageCapabilities {
        // Simplified detection
        let is_ssd = true; // Assume modern systems use SSD
        let sequential_read_mbps = if is_ssd { 500 } else { 100 };

        StorageCapabilities {
            total_gb: 512.0,
            available_gb: 256.0,
            is_ssd,
            sequential_read_mbps,
        }
    }

    fn detect_network() -> NetworkCapabilities {
        NetworkCapabilities {
            has_wifi: true,
            has_5g: false,
            has_ethernet: true,
            estimated_bandwidth_mbps: 100,
        }
    }

    fn detect_battery() -> BatteryCapabilities {
        // Check for battery on laptops
        #[cfg(target_os = "macos")]
        let has_battery = {
            let status = std::process::Command::new("pmset")
                .arg("-g")
                .arg("batt")
                .output();
            status.is_ok()
        };

        #[cfg(not(target_os = "macos"))]
        let has_battery = false;

        BatteryCapabilities {
            has_battery,
            is_plugged_in: true,
            battery_level_percent: 100,
            thermal_throttling: false,
        }
    }

    /// Classify device tier
    pub fn device_tier(&self) -> DeviceTier {
        if self.memory.total_ram_gb >= 32.0 && self.cpu.cores >= 8 {
            if self.gpu.has_gpu && self.gpu.gpu_type != GpuType::None {
                DeviceTier::UltraCapability
            } else {
                DeviceTier::HighCapability
            }
        } else if self.memory.total_ram_gb >= 8.0 && self.cpu.cores >= 4 {
            DeviceTier::Standard
        } else if self.memory.total_ram_gb >= 4.0 {
            DeviceTier::LowMemory
        } else {
            DeviceTier::Constrained
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTier {
    UltraCapability,
    HighCapability,
    Standard,
    LowMemory,
    Constrained,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_detection() {
        let caps = DeviceCapabilities::detect().unwrap();
        assert!(caps.cpu.cores > 0);
        assert!(caps.memory.total_ram_gb > 0.0);
    }

    #[test]
    fn test_device_tier_detection() {
        let caps = DeviceCapabilities::detect().unwrap();
        let tier = caps.device_tier();
        assert_ne!(tier, DeviceTier::Constrained); // Most systems should have >2GB
    }

    #[test]
    fn test_cpu_detection() {
        let cpu = DeviceCapabilities::detect_cpu();
        assert!(cpu.cores > 0);
        assert!(!cpu.architecture.is_empty());
    }

    #[test]
    fn test_memory_detection() {
        let mem = DeviceCapabilities::detect_memory();
        assert!(mem.total_ram_gb > 0.0);
        assert!(mem.available_ram_gb <= mem.total_ram_gb);
    }
}
