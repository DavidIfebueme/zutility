use anyhow::Result;

use zutility_be::{config::AppConfig, http, integrations::zcash, observability};

#[tokio::main]
async fn main() -> Result<()> {
    observability::init_tracing();
    let config = AppConfig::from_env()?;
    config.validate()?;
    zcash::validate_runtime_network_policy(&config)?;

    let app = http::build_router(&config);
    let listener = tokio::net::TcpListener::bind(config.http_bind_addr).await?;
    tracing::info!(bind = %config.http_bind_addr, env = ?config.app_env, "backend http server started");
    axum::serve(listener, app).await?;

    Ok(())
}
