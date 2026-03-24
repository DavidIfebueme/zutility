use anyhow::Result;

use zutility_be::{config::AppConfig, observability};

#[tokio::main]
async fn main() -> Result<()> {
    observability::init_tracing();
    let config = AppConfig::from_env()?;
    config.validate()?;
    tracing::info!(bind = %config.http_bind_addr, env = ?config.app_env, "backend bootstrap initialized");
    Ok(())
}
