use std::net::SocketAddr;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Dev,
    Staging,
    Prod,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ZcashRpcMode {
    Unix,
    Tcp,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ZcashNetwork {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub http_bind_addr: SocketAddr,
    pub database_url: String,
    pub order_token_hmac_secret: SecretString,
    pub ip_hash_secret: SecretString,
    pub vtpass_base_url: String,
    pub vtpass_api_key: SecretString,
    pub vtpass_secret_key: SecretString,
    pub zcash_rpc_mode: ZcashRpcMode,
    pub zcash_rpc_socket_path: String,
    pub zcash_rpc_url: String,
    pub zcash_rpc_user: SecretString,
    pub zcash_rpc_password: SecretString,
    pub zcash_network: ZcashNetwork,
    pub required_confs_transparent: u16,
    pub required_confs_shielded: u16,
    pub order_expiry_minutes: u16,
    pub rate_lock_minutes: u16,
    pub sweep_threshold_zec: Decimal,
    pub signing_service_url: String,
    pub signing_service_hmac_secret: SecretString,
    pub rate_source_timeout_ms: u64,
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
        ensure_non_empty("DATABASE_URL", &self.database_url)?;
        ensure_non_empty_secret("ORDER_TOKEN_HMAC_SECRET", &self.order_token_hmac_secret)?;
        ensure_non_empty_secret("IP_HASH_SECRET", &self.ip_hash_secret)?;
        ensure_non_empty("VTPASS_BASE_URL", &self.vtpass_base_url)?;
        ensure_non_empty_secret("VTPASS_API_KEY", &self.vtpass_api_key)?;
        ensure_non_empty_secret("VTPASS_SECRET_KEY", &self.vtpass_secret_key)?;
        ensure_non_empty_secret("ZCASH_RPC_USER", &self.zcash_rpc_user)?;
        ensure_non_empty_secret("ZCASH_RPC_PASSWORD", &self.zcash_rpc_password)?;
        ensure_non_empty("SIGNING_SERVICE_URL", &self.signing_service_url)?;
        ensure_non_empty_secret(
            "SIGNING_SERVICE_HMAC_SECRET",
            &self.signing_service_hmac_secret,
        )?;

        match self.zcash_rpc_mode {
            ZcashRpcMode::Unix => {
                ensure_non_empty("ZCASH_RPC_SOCKET_PATH", &self.zcash_rpc_socket_path)?
            }
            ZcashRpcMode::Tcp => ensure_non_empty("ZCASH_RPC_URL", &self.zcash_rpc_url)?,
        }

        if self.required_confs_transparent == 0 {
            anyhow::bail!("REQUIRED_CONFS_TRANSPARENT must be greater than 0");
        }
        if self.required_confs_shielded == 0 {
            anyhow::bail!("REQUIRED_CONFS_SHIELDED must be greater than 0");
        }
        if self.order_expiry_minutes == 0 {
            anyhow::bail!("ORDER_EXPIRY_MINUTES must be greater than 0");
        }
        if self.rate_lock_minutes == 0 {
            anyhow::bail!("RATE_LOCK_MINUTES must be greater than 0");
        }
        if self.rate_lock_minutes > self.order_expiry_minutes {
            anyhow::bail!("RATE_LOCK_MINUTES cannot exceed ORDER_EXPIRY_MINUTES");
        }
        if self.sweep_threshold_zec <= Decimal::ZERO {
            anyhow::bail!("SWEEP_THRESHOLD_ZEC must be greater than 0");
        }
        if self.rate_source_timeout_ms == 0 {
            anyhow::bail!("RATE_SOURCE_TIMEOUT_MS must be greater than 0");
        }

        Ok(())
    }
}

fn ensure_non_empty(key: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{key} must not be empty");
    }
    Ok(())
}

fn ensure_non_empty_secret(key: &str, value: &SecretString) -> Result<()> {
    if value.expose_secret().trim().is_empty() {
        anyhow::bail!("{key} must not be empty");
    }
    Ok(())
}
