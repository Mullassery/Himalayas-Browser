use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use uuid::Uuid;
use tracing::info;

mod lifecycle;

pub use lifecycle::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub environment: String,
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
        }
    }
}

pub struct Daemon {
    config: Config,
    health_server: SocketAddr,
}

impl Daemon {
    pub async fn new(config: Config) -> Result<Self> {
        config.validate()?;

        let addr = format!("{}:{}", config.host, config.port)
            .parse::<SocketAddr>()?;

        info!(daemon_id = %config.id, address = %addr, environment = %config.environment, "Daemon created");

        Ok(Self {
            config,
            health_server: addr,
        })
    }

    pub fn health_addr(&self) -> SocketAddr {
        self.health_server
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting health monitoring server on {}", self.health_server);
        info!("Daemon running - Phase 0 Foundation");
        info!("Daemon ID: {}", self.config.id);
        info!("Environment: {}", self.config.environment);

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
