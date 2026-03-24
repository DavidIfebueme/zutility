use std::net::SocketAddr;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Dev,
    Staging,
    Prod,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub http_bind_addr: SocketAddr,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .context("failed to build config from environment")?;
        Self::from_config(config)
    }

    pub fn from_config(config: config::Config) -> Result<Self> {
        config
            .try_deserialize::<AppConfig>()
            .context("failed to deserialize environment config")
    }

    pub fn validate(&self) -> Result<()> {
        if self.database_url.trim().is_empty() {
            anyhow::bail!("DATABASE_URL must not be empty");
        }
        Ok(())
    }
}
