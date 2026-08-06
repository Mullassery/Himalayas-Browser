use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::FusedLocation;

/// GNSS spoofing detection engine
/// Protects against fake location signals using behavioral analysis
pub struct SpoofingDetector {
    last_location: std::sync::Mutex<Option<FusedLocation>>,
    max_speed_ms: f32,           // Max realistic speed (m/s)
    min_accuracy_m: f32,         // Min realistic accuracy
    jump_threshold_m: f32,       // Max distance between consecutive fixes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofingAnalysis {
    pub is_spoofed: bool,
    pub confidence: f32,
    pub anomalies: Vec<SpoofingAnomaly>,
    pub risk_score: f32,         // 0.0-1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SpoofingAnomaly {
    ImpossibleSpeed,
    UnrealisticAccuracy,
    LocationJump,
    InconsistentSignals,
    WeakSignalStrength,
    UnusualDOP,                  // Dilution of Precision
    ConstellationMismatch,
}

impl SpoofingDetector {
    pub fn new() -> Result<Self> {
        debug!("Initializing Spoofing Detector");

        Ok(Self {
            last_location: std::sync::Mutex::new(None),
            max_speed_ms: 300.0,   // ~1080 km/h (roughly max jet speed)
            min_accuracy_m: 1.0,   // Realistic minimum accuracy
            jump_threshold_m: 100_000.0, // 100km jump = suspicious
        })
    }

    /// Check if location appears to be spoofed
    pub fn is_spoofed(&self, location: &FusedLocation) -> Result<bool> {
        debug!("Analyzing location for spoofing indicators");

        let analysis = self.analyze(location)?;
        Ok(analysis.is_spoofed)
    }

    /// Detailed spoofing analysis
    pub fn analyze(&self, location: &FusedLocation) -> Result<SpoofingAnalysis> {
        let mut anomalies = Vec::new();
        let mut risk_score = 0.0f32;

        // Check 1: Impossible speed
        if let Ok(last_loc) = self.last_location.lock() {
            if let Some(prev) = last_loc.as_ref() {
                let distance = self.calculate_distance(
                    prev.latitude,
                    prev.longitude,
                    location.latitude,
                    location.longitude,
                );

                let time_diff = location.timestamp_s.saturating_sub(prev.timestamp_s).max(1);
                let speed = distance / time_diff as f32;

                if speed > self.max_speed_ms {
                    anomalies.push(SpoofingAnomaly::ImpossibleSpeed);
                    risk_score += 0.3;
                    warn!(
                        "⚠️ Impossible speed detected: {:.1} m/s (max: {:.1})",
                        speed, self.max_speed_ms
                    );
                }

                // Check 2: Suspicious location jump
                if distance > self.jump_threshold_m {
                    anomalies.push(SpoofingAnomaly::LocationJump);
                    risk_score += 0.2;
                    warn!("⚠️ Suspicious location jump: {:.0}m", distance);
                }
            }
        }

        // Check 3: Unrealistic accuracy claims
        if location.accuracy_m < self.min_accuracy_m {
            anomalies.push(SpoofingAnomaly::UnrealisticAccuracy);
            risk_score += 0.15;
            warn!("⚠️ Unrealistic accuracy: {:.1}m (min: {:.1}m)", location.accuracy_m, self.min_accuracy_m);
        }

        // Check 4: Overly perfect accuracy
        if location.accuracy_m < 1.0 && location.confidence < 0.95 {
            anomalies.push(SpoofingAnomaly::InconsistentSignals);
            risk_score += 0.1;
            warn!("⚠️ Inconsistent signal quality");
        }

        // Check 5: Low confidence but high accuracy claim
        if location.accuracy_m < 10.0 && location.confidence < 0.7 {
            anomalies.push(SpoofingAnomaly::InconsistentSignals);
            risk_score += 0.15;
        }

        // Update last location for next check
        let _ = self.last_location.lock().map(|mut l| {
            *l = Some(location.clone());
        });

        let is_spoofed = risk_score > 0.4;
        if is_spoofed {
            warn!("🚨 SPOOFING ALERT: Risk score {:.1}%", risk_score * 100.0);
        }

        Ok(SpoofingAnalysis {
            is_spoofed,
            confidence: 0.8 + (risk_score * 0.2).min(0.2), // 0.8-1.0
            anomalies,
            risk_score,
        })
    }

    /// Behavioral baseline analysis
    pub fn get_baseline_stats(&self) -> SpoofingBaseline {
        SpoofingBaseline {
            typical_accuracy_m: 5.0,
            typical_speed_ms: 15.0,      // Walking speed
            typical_jump_m: 500.0,       // 500m
            suspicious_accuracy_m: 0.5,
            suspicious_speed_ms: 200.0,  // Supersonic
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
pub struct SpoofingBaseline {
    pub typical_accuracy_m: f32,
    pub typical_speed_ms: f32,
    pub typical_jump_m: f32,
    pub suspicious_accuracy_m: f32,
    pub suspicious_speed_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::location_fusion::FusionWeights;

    #[test]
    fn test_spoofing_detector_creation() {
        let detector = SpoofingDetector::new().unwrap();
        assert!(detector.max_speed_ms > 0.0);
    }

    #[test]
    fn test_impossible_speed_detection() {
        let detector = SpoofingDetector::new().unwrap();

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

        let _ = detector.analyze(&loc);

        // Second location impossibly far away in 1 second
        let spoofed_loc = FusedLocation {
            latitude: 38.7749, // ~111km north
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
            timestamp_s: 1001,
        };

        let analysis = detector.analyze(&spoofed_loc).unwrap();
        assert!(analysis.is_spoofed || analysis.risk_score > 0.2);
    }

    #[test]
    fn test_unrealistic_accuracy() {
        let detector = SpoofingDetector::new().unwrap();

        let loc = FusedLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_m: 0.0,
            accuracy_m: 0.01, // Unrealistically accurate
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

        let analysis = detector.analyze(&loc).unwrap();
        assert!(analysis.anomalies.iter().any(|a| matches!(a, SpoofingAnomaly::UnrealisticAccuracy)));
    }

    #[test]
    fn test_baseline_stats() {
        let detector = SpoofingDetector::new().unwrap();
        let baseline = detector.get_baseline_stats();
        assert!(baseline.typical_accuracy_m > 0.0);
        assert!(baseline.suspicious_speed_ms > baseline.typical_speed_ms);
    }
}
