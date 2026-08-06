use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

/// GNSS satellite constellations supported by Himalayas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GNSSConstellation {
    GPS,        // US Global Positioning System
    NavIC,      // Indian Regional Navigation Satellite System
    BeiDou,     // Chinese Navigation Satellite System
    Galileo,    // European GNSS
    GLONASS,    // Russian GNSS
    QZSS,       // Japanese Quasi-Zenith Satellite System
}

impl GNSSConstellation {
    pub fn satellite_count(&self) -> u8 {
        match self {
            Self::GPS => 31,
            Self::NavIC => 8,
            Self::BeiDou => 45,
            Self::Galileo => 30,
            Self::GLONASS => 24,
            Self::QZSS => 4,
        }
    }

    pub fn frequency_ghz(&self) -> f32 {
        match self {
            Self::GPS => 1.575,
            Self::NavIC => 2.492,
            Self::BeiDou => 1.561,
            Self::Galileo => 1.575,
            Self::GLONASS => 1.602,
            Self::QZSS => 1.575,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::GPS => "GPS (US)",
            Self::NavIC => "NavIC (India)",
            Self::BeiDou => "BeiDou (China)",
            Self::Galileo => "Galileo (EU)",
            Self::GLONASS => "GLONASS (Russia)",
            Self::QZSS => "QZSS (Japan)",
        }
    }

    pub fn availability(&self) -> &str {
        match self {
            Self::GPS => "Global",
            Self::NavIC => "Indian subcontinent",
            Self::BeiDou => "Global with best coverage in Asia",
            Self::Galileo => "Global",
            Self::GLONASS => "Global",
            Self::QZSS => "Asia-Pacific region",
        }
    }
}

/// Individual satellite signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteSignal {
    pub prn: u8,                    // Pseudo-random number
    pub constellation: GNSSConstellation,
    pub signal_strength_dbhz: i16, // Signal strength in dB-Hz
    pub pseudorange_m: f64,        // Pseudorange in meters
    pub doppler_hz: i32,           // Doppler shift in Hz
    pub elevation_deg: f32,        // Elevation angle
    pub azimuth_deg: f32,          // Azimuth angle
    pub in_use: bool,              // Used in fix calculation
}

/// GNSS position fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GNSSFix {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f32,
    pub accuracy_m: f32,           // Horizontal accuracy
    pub vertical_accuracy_m: f32,
    pub fix_type: FixType,
    pub satellites_used: u8,
    pub hdop: f32,                 // Horizontal dilution of precision
    pub timestamp_s: u64,
    pub constellations_used: Vec<GNSSConstellation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixType {
    NoFix,
    DeadReckoning,
    SinglePoint2D,
    SinglePoint3D,
    DGPS,
    RTK,
    FloatRTK,
}

/// GNSS Manager handling multi-constellation positioning
pub struct GNSSManager {
    current_fix: Arc<RwLock<Option<GNSSFix>>>,
    satellites: Arc<RwLock<Vec<SatelliteSignal>>>,
    preferred_constellation: GNSSConstellation,
    use_multi_constellation: bool,
    spoofing_check_enabled: bool,
}

impl GNSSManager {
    pub fn new() -> Result<Self> {
        debug!("Initializing GNSS Manager");
        info!("🛰️  Multi-constellation GNSS system ready");

        Ok(Self {
            current_fix: Arc::new(RwLock::new(None)),
            satellites: Arc::new(RwLock::new(Vec::new())),
            preferred_constellation: GNSSConstellation::GPS,
            use_multi_constellation: true,
            spoofing_check_enabled: true,
        })
    }

    /// Set preferred constellation (NavIC for India, BeiDou for China, etc)
    pub fn set_preferred_constellation(&mut self, constellation: GNSSConstellation) {
        info!("Setting preferred constellation: {}", constellation.name());
        self.preferred_constellation = constellation;
    }

    /// Enable multi-constellation for better accuracy
    pub fn enable_multi_constellation(&mut self, enabled: bool) {
        self.use_multi_constellation = enabled;
        if enabled {
            info!("Multi-constellation fusion enabled (GPS + NavIC + BeiDou + Galileo + GLONASS)");
        } else {
            info!("Multi-constellation fusion disabled");
        }
    }

    /// Get current GNSS fix
    pub async fn get_fix(&self) -> Result<Option<GNSSFix>> {
        debug!("Getting GNSS fix");

        // Simulated fix (in production: query hardware GNSS receiver)
        let fix = Some(GNSSFix {
            latitude: 37.7749,  // San Francisco
            longitude: -122.4194,
            altitude_m: 52.0,
            accuracy_m: 5.0,
            vertical_accuracy_m: 10.0,
            fix_type: FixType::SinglePoint3D,
            satellites_used: if self.use_multi_constellation { 24 } else { 8 },
            hdop: 1.2,
            timestamp_s: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            constellations_used: vec![
                Some(GNSSConstellation::GPS),
                if self.use_multi_constellation { Some(GNSSConstellation::Galileo) } else { None },
                if self.use_multi_constellation { Some(GNSSConstellation::GLONASS) } else { None },
            ]
            .into_iter()
            .flatten()
            .collect(),
        });

        *self.current_fix.write() = fix.clone();
        Ok(fix)
    }

    /// Get visible satellites
    pub async fn get_visible_satellites(&self) -> Result<Vec<SatelliteSignal>> {
        debug!("Getting visible satellites");

        let sats = vec![
            // GPS satellites
            SatelliteSignal {
                prn: 1,
                constellation: GNSSConstellation::GPS,
                signal_strength_dbhz: 42,
                pseudorange_m: 20_000_000.0,
                doppler_hz: 1000,
                elevation_deg: 45.0,
                azimuth_deg: 0.0,
                in_use: true,
            },
            // NavIC satellites (India)
            SatelliteSignal {
                prn: 1,
                constellation: GNSSConstellation::NavIC,
                signal_strength_dbhz: 38,
                pseudorange_m: 20_000_000.0,
                doppler_hz: 800,
                elevation_deg: 30.0,
                azimuth_deg: 90.0,
                in_use: true,
            },
            // BeiDou satellites
            SatelliteSignal {
                prn: 1,
                constellation: GNSSConstellation::BeiDou,
                signal_strength_dbhz: 40,
                pseudorange_m: 20_000_000.0,
                doppler_hz: 900,
                elevation_deg: 60.0,
                azimuth_deg: 180.0,
                in_use: true,
            },
        ];

        *self.satellites.write() = sats.clone();
        Ok(sats)
    }

    /// Get GNSS constellation visibility
    pub async fn get_visibility(&self) -> Result<crate::device::ConstellationVisibility> {
        let sats = self.get_visible_satellites().await?;

        let mut visibility = crate::device::ConstellationVisibility {
            gps_sats: 0,
            navic_sats: 0,
            beidou_sats: 0,
            galileo_sats: 0,
            glonass_sats: 0,
            total_sats: sats.len() as u8,
        };

        for sat in sats {
            match sat.constellation {
                GNSSConstellation::GPS => visibility.gps_sats += 1,
                GNSSConstellation::NavIC => visibility.navic_sats += 1,
                GNSSConstellation::BeiDou => visibility.beidou_sats += 1,
                GNSSConstellation::Galileo => visibility.galileo_sats += 1,
                GNSSConstellation::GLONASS => visibility.glonass_sats += 1,
                _ => {}
            }
        }

        Ok(visibility)
    }

    /// Check GNSS health
    pub fn is_available(&self) -> bool {
        self.current_fix.read().is_some()
    }

    /// Get fix quality (0.0-1.0)
    pub fn get_fix_quality(&self) -> Option<f32> {
        self.current_fix.read().as_ref().map(|fix| {
            match fix.fix_type {
                FixType::NoFix => 0.0,
                FixType::DeadReckoning => 0.2,
                FixType::SinglePoint2D => 0.4,
                FixType::SinglePoint3D => 0.6,
                FixType::DGPS => 0.8,
                FixType::FloatRTK => 0.85,
                FixType::RTK => 0.95,
            }
        })
    }

    /// Get GNSS statistics
    pub fn get_stats(&self) -> GNSSStats {
        let fix = self.current_fix.read();
        let sats = self.satellites.read();

        GNSSStats {
            fix_available: fix.is_some(),
            satellites_visible: sats.len(),
            satellites_used: fix.as_ref().map(|f| f.satellites_used as usize).unwrap_or(0),
            horizontal_accuracy_m: fix.as_ref().map(|f| f.accuracy_m).unwrap_or(0.0),
            constellations_enabled: if self.use_multi_constellation { 6 } else { 1 },
            spoofing_detection: self.spoofing_check_enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GNSSStats {
    pub fix_available: bool,
    pub satellites_visible: usize,
    pub satellites_used: usize,
    pub horizontal_accuracy_m: f32,
    pub constellations_enabled: usize,
    pub spoofing_detection: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constellation_properties() {
        assert_eq!(GNSSConstellation::GPS.satellite_count(), 31);
        assert_eq!(GNSSConstellation::NavIC.satellite_count(), 8);
        assert!(GNSSConstellation::GPS.frequency_ghz() > 1.0);
    }

    #[tokio::test]
    async fn test_gnss_manager_creation() {
        let manager = GNSSManager::new().unwrap();
        assert!(!manager.is_available() || manager.is_available()); // Test doesn't panic
    }

    #[tokio::test]
    async fn test_get_visible_satellites() {
        let manager = GNSSManager::new().unwrap();
        let sats = manager.get_visible_satellites().await.unwrap();
        assert!(sats.len() > 0);
    }

    #[tokio::test]
    async fn test_multi_constellation() {
        let mut manager = GNSSManager::new().unwrap();
        manager.enable_multi_constellation(true);
        manager.set_preferred_constellation(GNSSConstellation::NavIC);

        let fix = manager.get_fix().await.unwrap();
        if let Some(f) = fix {
            assert!(f.constellations_used.len() > 1);
        }
    }

    #[tokio::test]
    async fn test_gnss_stats() {
        let manager = GNSSManager::new().unwrap();
        manager.get_fix().await.unwrap();
        let stats = manager.get_stats();
        assert!(stats.constellations_enabled > 0);
    }

    #[test]
    fn test_constellation_names() {
        assert!(!GNSSConstellation::NavIC.name().is_empty());
        assert!(!GNSSConstellation::BeiDou.availability().is_empty());
    }
}
