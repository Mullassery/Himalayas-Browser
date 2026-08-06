use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;
use tracing::info;

/// Browser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub browser: BrowserSettings,
    pub privacy: PrivacySettings,
    pub performance: PerformanceSettings,
    pub developer: DeveloperSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub user_agent: String,
    pub accept_language: String,
    pub timezone: String,
    pub max_tabs: usize,
    pub default_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub do_not_track: bool,
    pub block_third_party_cookies: bool,
    pub clear_history_on_close: bool,
    pub block_trackers: bool,
    pub fingerprint_protection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub enable_caching: bool,
    pub cache_size_mb: usize,
    pub enable_compression: bool,
    pub max_memory_mb: usize,
    pub enable_gpu_acceleration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperSettings {
    pub enable_devtools: bool,
    pub enable_console: bool,
    pub log_level: String,
    pub enable_network_monitor: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            browser: BrowserSettings {
                user_agent: "Himalayas/0.1.0 (Agent-Native Browser)".to_string(),
                accept_language: "en-US,en;q=0.9".to_string(),
                timezone: "UTC".to_string(),
                max_tabs: 50,
                default_url: "about:blank".to_string(),
            },
            privacy: PrivacySettings {
                do_not_track: true,
                block_third_party_cookies: true,
                clear_history_on_close: false,
                block_trackers: true,
                fingerprint_protection: true,
            },
            performance: PerformanceSettings {
                enable_caching: true,
                cache_size_mb: 500,
                enable_compression: true,
                max_memory_mb: 2048,
                enable_gpu_acceleration: true,
            },
            developer: DeveloperSettings {
                enable_devtools: false,
                enable_console: false,
                log_level: "info".to_string(),
                enable_network_monitor: false,
            },
        }
    }
}

impl BrowserConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config = toml::from_str(&content)?;
            info!("Config loaded: {}", path.display());
            Ok(config)
        } else {
            info!("Config not found, using defaults");
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        info!("Config saved: {}", path.display());
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.browser.max_tabs == 0 {
            anyhow::bail!("max_tabs must be > 0");
        }
        if self.performance.max_memory_mb < 512 {
            anyhow::bail!("max_memory_mb should be >= 512");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BrowserConfig::default();
        assert!(config.privacy.do_not_track);
        assert!(config.performance.enable_caching);
        assert_eq!(config.browser.max_tabs, 50);
    }

    #[test]
    fn test_config_validate() {
        let config = BrowserConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_invalid() {
        let mut config = BrowserConfig::default();
        config.browser.max_tabs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = BrowserConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: BrowserConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.browser.max_tabs, config.browser.max_tabs);
    }
}
