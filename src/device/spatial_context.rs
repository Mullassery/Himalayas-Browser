use serde::{Deserialize, Serialize};
use tracing::debug;

use super::FusedLocation;

/// Spatial context for agent-aware operations
/// Provides location, movement, and spatial reasoning for AI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialContext {
    pub current_location: Option<LocationInfo>,
    pub movement_state: MovementState,
    pub environment: EnvironmentInfo,
    pub agent_constraints: AgentConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f32,
    pub accuracy_m: f32,
    pub place_name: Option<String>,
    pub timestamp_s: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MovementState {
    Stationary,
    Slow,        // Walking pace
    Moderate,    // Vehicle pace
    Fast,        // Highway pace
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub satellite_count: u8,
    pub fix_quality: f32,
    pub indoor_likelihood: f32,   // 0.0-1.0
    pub urban_density: f32,       // 0.0-1.0 (urban vs rural)
    pub connectivity: ConnectivityInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityInfo {
    pub has_gnss: bool,
    pub has_wifi: bool,
    pub has_cellular: bool,
    pub has_ble: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConstraints {
    pub max_operational_radius_km: f32,
    pub geofence_enabled: bool,
    pub geofence_areas: Vec<GeofenceArea>,
    pub location_sharing_allowed: bool,
    pub location_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeofenceArea {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub radius_m: f32,
    pub allow_entry: bool,
}

impl SpatialContext {
    pub fn new() -> Self {
        debug!("Initializing Spatial Context for Agents");

        Self {
            current_location: None,
            movement_state: MovementState::Unknown,
            environment: EnvironmentInfo {
                satellite_count: 0,
                fix_quality: 0.0,
                indoor_likelihood: 0.0,
                urban_density: 0.0,
                connectivity: ConnectivityInfo {
                    has_gnss: false,
                    has_wifi: false,
                    has_cellular: false,
                    has_ble: false,
                },
            },
            agent_constraints: AgentConstraints {
                max_operational_radius_km: 10.0,
                geofence_enabled: false,
                geofence_areas: Vec::new(),
                location_sharing_allowed: false,
                location_logging: true,
            },
        }
    }

    /// Update context with new location
    pub fn update_location(&mut self, location: FusedLocation) {
        let new_state = if location.accuracy_m > 100.0 {
            MovementState::Unknown
        } else if location.accuracy_m < 1.0 {
            MovementState::Stationary
        } else {
            MovementState::Moderate
        };

        self.current_location = Some(LocationInfo {
            latitude: location.latitude,
            longitude: location.longitude,
            altitude_m: location.altitude_m,
            accuracy_m: location.accuracy_m,
            place_name: None,
            timestamp_s: location.timestamp_s,
        });

        self.movement_state = new_state;
        self.environment.fix_quality = location.confidence;
    }

    /// Check if location is within geofence
    pub fn is_within_geofence(&self, latitude: f64, longitude: f64) -> bool {
        if !self.agent_constraints.geofence_enabled {
            return true;
        }

        for area in &self.agent_constraints.geofence_areas {
            let dist = self.calculate_distance(latitude, longitude, area.latitude, area.longitude);
            if dist <= area.radius_m {
                return area.allow_entry;
            }
        }

        true
    }

    /// Add geofence area
    pub fn add_geofence(&mut self, area: GeofenceArea) {
        self.agent_constraints.geofence_areas.push(area);
        self.agent_constraints.geofence_enabled = true;
    }

    /// Get agent privacy summary
    pub fn privacy_summary(&self) -> PrivacySummary {
        PrivacySummary {
            location_privacy: if self.agent_constraints.location_sharing_allowed {
                "Sharing enabled"
            } else {
                "Local only"
            },
            logging_enabled: self.agent_constraints.location_logging,
            geofencing: self.agent_constraints.geofence_enabled,
            geofence_count: self.agent_constraints.geofence_areas.len(),
        }
    }

    /// Get operational capabilities based on location
    pub fn get_agent_capabilities(&self) -> AgentCapabilities {
        let has_location = self.current_location.is_some();
        let high_accuracy = self.current_location.as_ref().map(|l| l.accuracy_m < 10.0).unwrap_or(false);

        AgentCapabilities {
            location_aware: has_location,
            high_precision_positioning: high_accuracy,
            indoor_supported: self.environment.indoor_likelihood < 0.3,
            outdoor_supported: self.environment.urban_density > 0.3,
            multi_constellation_available: self.environment.satellite_count > 12,
        }
    }

    /// Estimate indoor/outdoor likelihood
    pub fn detect_environment(&mut self) {
        if let Some(ref loc) = self.current_location {
            // Accuracy under 5m suggests outdoor with clear sky
            self.environment.indoor_likelihood = if loc.accuracy_m < 5.0 { 0.1 } else { 0.7 };
        }
    }

    fn calculate_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        (EARTH_RADIUS_M * c) as f32
    }
}

#[derive(Debug, Clone)]
pub struct PrivacySummary {
    pub location_privacy: &'static str,
    pub logging_enabled: bool,
    pub geofencing: bool,
    pub geofence_count: usize,
}

#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    pub location_aware: bool,
    pub high_precision_positioning: bool,
    pub indoor_supported: bool,
    pub outdoor_supported: bool,
    pub multi_constellation_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::location_fusion::FusionWeights;

    #[test]
    fn test_spatial_context_creation() {
        let context = SpatialContext::new();
        assert!(context.current_location.is_none());
    }

    #[test]
    fn test_update_location() {
        let mut context = SpatialContext::new();

        let loc = FusedLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_m: 0.0,
            accuracy_m: 5.0,
            confidence: 0.9,
            sensor_fusion_weights: FusionWeights {
                gnss_weight: 1.0,
                wifi_weight: 0.0,
                ble_weight: 0.0,
                imu_weight: 0.0,
                cellular_weight: 0.0,
            },
            timestamp_s: 1000,
        };

        context.update_location(loc);
        assert!(context.current_location.is_some());
    }

    #[test]
    fn test_geofence() {
        let mut context = SpatialContext::new();

        let area = GeofenceArea {
            name: "Office".to_string(),
            latitude: 37.7749,
            longitude: -122.4194,
            radius_m: 500.0,
            allow_entry: true,
        };

        context.add_geofence(area);
        assert!(context.agent_constraints.geofence_enabled);
        assert!(context.is_within_geofence(37.7749, -122.4194));
    }

    #[test]
    fn test_agent_capabilities() {
        let mut context = SpatialContext::new();

        let loc = FusedLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_m: 0.0,
            accuracy_m: 3.0,
            confidence: 0.95,
            sensor_fusion_weights: FusionWeights {
                gnss_weight: 1.0,
                wifi_weight: 0.0,
                ble_weight: 0.0,
                imu_weight: 0.0,
                cellular_weight: 0.0,
            },
            timestamp_s: 1000,
        };

        context.update_location(loc);
        let caps = context.get_agent_capabilities();
        assert!(caps.location_aware);
        assert!(caps.high_precision_positioning);
    }

    #[test]
    fn test_privacy_summary() {
        let context = SpatialContext::new();
        let summary = context.privacy_summary();
        assert_eq!(summary.geofence_count, 0);
    }
}
