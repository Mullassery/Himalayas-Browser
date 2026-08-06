pub mod gnss;
pub mod location_fusion;
pub mod location_memory;
pub mod spoofing_detection;
pub mod spatial_context;

pub use gnss::{GNSSManager, GNSSConstellation, SatelliteSignal};
pub use location_fusion::{LocationFusionEngine, FusedLocation, SensorReading};
pub use location_memory::{LocationMemoryGraph, LocationNode};
pub use spoofing_detection::SpoofingDetector;
pub use spatial_context::SpatialContext;

use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug};

/// Device integration orchestrator
/// Coordinates GNSS, sensor fusion, location memory, and spatial intelligence
pub struct DeviceIntegrationManager {
    gnss: Arc<GNSSManager>,
    fusion_engine: Arc<LocationFusionEngine>,
    location_memory: Arc<LocationMemoryGraph>,
    spoofing_detector: Arc<SpoofingDetector>,
    spatial_context: Arc<RwLock<SpatialContext>>,
    enabled: bool,
}

impl DeviceIntegrationManager {
    pub async fn new() -> Result<Self> {
        info!("🧭 Initializing Device Integration Manager");

        let gnss = Arc::new(GNSSManager::new()?);
        let fusion_engine = Arc::new(LocationFusionEngine::new()?);
        let location_memory = Arc::new(LocationMemoryGraph::new()?);
        let spoofing_detector = Arc::new(SpoofingDetector::new()?);
        let spatial_context = Arc::new(RwLock::new(SpatialContext::new()));

        Ok(Self {
            gnss,
            fusion_engine,
            location_memory,
            spoofing_detector,
            spatial_context,
            enabled: true,
        })
    }

    /// Get current fused location from all available sensors
    pub async fn get_location(&self) -> Result<Option<FusedLocation>> {
        if !self.enabled {
            return Ok(None);
        }

        debug!("Retrieving fused location");

        // Get GNSS fix
        let gnss_fix = self.gnss.get_fix().await?;

        // Get other sensor readings
        let wifi_reading = self.fusion_engine.get_wifi_reading().await?;
        let ble_reading = self.fusion_engine.get_ble_reading().await?;
        let imu_reading = self.fusion_engine.get_imu_reading().await?;

        // Fuse all readings
        let mut fused = self.fusion_engine.fuse_sensors(
            gnss_fix.as_ref(),
            wifi_reading,
            ble_reading,
            imu_reading,
        )?;

        // Check for spoofing
        if let Some(ref loc) = fused {
            if self.spoofing_detector.is_spoofed(loc)? {
                info!("⚠️ Potential GNSS spoofing detected, using fallback");
                fused = self.fusion_engine.get_fallback_location().await?;
            }
        }

        // Update location memory
        if let Some(ref loc) = fused {
            self.location_memory.add_location(loc.clone())?;

            // Update spatial context for agents
            let mut context = self.spatial_context.write();
            context.update_location(loc.clone());
        }

        Ok(fused)
    }

    /// Get spatial context for agent operations
    pub fn get_spatial_context(&self) -> SpatialContext {
        self.spatial_context.read().clone()
    }

    /// Query location history
    pub fn query_location_history(&self, latitude: f64, longitude: f64, radius_km: f32) -> Result<Vec<LocationNode>> {
        self.location_memory.query_nearby(latitude, longitude, radius_km)
    }

    /// Get constellation visibility
    pub async fn get_constellation_visibility(&self) -> Result<ConstellationVisibility> {
        self.gnss.get_visibility().await
    }

    /// Enable/disable device integration
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            info!("Device integration enabled");
        } else {
            info!("Device integration disabled");
        }
    }

    /// Get integration status
    pub fn status(&self) -> DeviceStatus {
        DeviceStatus {
            enabled: self.enabled,
            gnss_available: self.gnss.is_available(),
            gnss_fix_quality: self.gnss.get_fix_quality(),
            fusion_engine_ready: true,
            location_memory_size: self.location_memory.size(),
            spoofing_detection_active: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub enabled: bool,
    pub gnss_available: bool,
    pub gnss_fix_quality: Option<f32>,
    pub fusion_engine_ready: bool,
    pub location_memory_size: usize,
    pub spoofing_detection_active: bool,
}

#[derive(Debug, Clone)]
pub struct ConstellationVisibility {
    pub gps_sats: u8,
    pub navic_sats: u8,
    pub beidou_sats: u8,
    pub galileo_sats: u8,
    pub glonass_sats: u8,
    pub total_sats: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_integration_creation() {
        let manager = DeviceIntegrationManager::new().await.unwrap();
        let status = manager.status();
        assert!(status.enabled);
    }

    #[tokio::test]
    async fn test_device_status() {
        let manager = DeviceIntegrationManager::new().await.unwrap();
        let status = manager.status();
        assert!(status.fusion_engine_ready);
        assert!(status.spoofing_detection_active);
    }

    #[tokio::test]
    async fn test_device_enable_disable() {
        let mut manager = DeviceIntegrationManager::new().await.unwrap();
        assert!(manager.status().enabled);

        manager.set_enabled(false);
        assert!(!manager.status().enabled);

        manager.set_enabled(true);
        assert!(manager.status().enabled);
    }
}
