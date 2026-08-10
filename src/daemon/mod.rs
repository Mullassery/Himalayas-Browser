use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use tracing::info;

use crate::browser::Browser;
use crate::health::HealthMonitor;
use crate::intelligence::device_detection::{DeviceCapabilities, DeviceTier};
use crate::metrics::MetricsCollector;
use crate::server::HealthServer;

mod lifecycle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub environment: String,
    /// Serve the browser UI at /app. `None` = auto-detect from device tier
    /// (on for Standard tier and above, off for LowMemory/Constrained).
    #[serde(default)]
    pub ui: Option<bool>,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.port == 0 {
            anyhow::bail!("Invalid port: port must be > 0");
        }
        if self.id.is_empty() {
            anyhow::bail!("Invalid daemon ID");
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            id: format!("himalayas-{}", Uuid::new_v4()),
            host: "127.0.0.1".to_string(),
            port: 8080,
            environment: "development".to_string(),
            ui: None,
        }
    }
}

/// Tiers with enough headroom to keep the UI (and per-tab isolated sessions)
/// on by default without being asked. Below this, both stay off until the
/// user opts in with --ui.
fn tier_supports_desktop_features(tier: DeviceTier) -> bool {
    matches!(
        tier,
        DeviceTier::Standard | DeviceTier::HighCapability | DeviceTier::UltraCapability
    )
}

pub struct Daemon {
    config: Config,
    health_server: SocketAddr,
    health_monitor: Arc<HealthMonitor>,
    metrics_collector: Arc<MetricsCollector>,
    device_tier: DeviceTier,
    ui_enabled: bool,
    /// The headless browser runtime backing the `/agent` HTTP API — see
    /// `src/server.rs`. One `Browser` per daemon process, shared across every
    /// agent session (each session still gets its own isolated `Session`
    /// inside it, same as `himalayas-desktop`'s native tabs).
    browser: Arc<Browser>,
}

impl Daemon {
    pub async fn new(config: Config) -> Result<Self> {
        config.validate()?;

        let addr = format!("{}:{}", config.host, config.port)
            .parse::<SocketAddr>()?;

        let device_tier = DeviceCapabilities::detect()
            .map(|caps| caps.device_tier())
            .unwrap_or(DeviceTier::Standard);

        let ui_enabled = config.ui.unwrap_or_else(|| {
            let auto = cfg!(feature = "desktop_ui") && tier_supports_desktop_features(device_tier);
            info!(?device_tier, ui_enabled = auto, "Browser UI auto-detected from device tier");
            auto
        }) && cfg!(feature = "desktop_ui");

        info!(daemon_id = %config.id, address = %addr, environment = %config.environment, ui_enabled, "Daemon created");

        Ok(Self {
            config,
            health_server: addr,
            health_monitor: Arc::new(HealthMonitor::new()),
            metrics_collector: Arc::new(MetricsCollector::new()),
            device_tier,
            ui_enabled,
            browser: Arc::new(Browser::new()?),
        })
    }

    pub fn ui_enabled(&self) -> bool {
        self.ui_enabled
    }

    pub fn device_tier(&self) -> DeviceTier {
        self.device_tier
    }

    pub fn health_addr(&self) -> SocketAddr {
        self.health_server
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn health_monitor(&self) -> Arc<HealthMonitor> {
        self.health_monitor.clone()
    }

    pub fn metrics_collector(&self) -> Arc<MetricsCollector> {
        self.metrics_collector.clone()
    }

    pub async fn run(self) -> Result<()> {
        info!("Daemon running - Phase 0 Foundation");
        info!("Daemon ID: {}", self.config.id);
        info!("Environment: {}", self.config.environment);

        // Create health server
        let health_server = HealthServer::new(
            self.health_monitor.clone(),
            self.metrics_collector.clone(),
            self.ui_enabled,
            self.device_tier,
            self.browser.clone(),
        );

        let server_addr = self.health_server;
        info!("Starting health monitoring server on {}", server_addr);

        // Spawn health server task
        let server_handle = tokio::spawn(async move {
            if let Err(e) = health_server.start(server_addr).await {
                eprintln!("Health server error: {}", e);
            }
        });

        // Keep daemon running
        server_handle.await?;

        Ok(())
    }
}
