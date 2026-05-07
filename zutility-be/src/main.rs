use anyhow::Result;
use std::net::SocketAddr;
use sqlx::postgres::PgPoolOptions;

use zutility_be::{config::AppConfig, http, integrations::zcash, observability, runtime};

#[tokio::main]
async fn main() -> Result<()> {
    observability::init_tracing();
    let config = AppConfig::from_env()?;
    let config = config.validate()?;
    zcash::validate_runtime_network_policy(&config)?;
    zcash::validate_rpc_socket_policy(&config)?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    let state = http::build_state(&config, None, pool.clone());
    runtime::start_background_workers(state.clone(), config.clone(), pool.clone());
    let app = http::build_router_from_state(state, true);
    let listener = tokio::net::TcpListener::bind(config.http_bind_addr).await?;
    tracing::info!(bind = %config.http_bind_addr, env = ?config.app_env, "backend http server started");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}
