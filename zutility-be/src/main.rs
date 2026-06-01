use std::sync::Arc;

use anyhow::Result;
use std::net::SocketAddr;
use sqlx::postgres::PgPoolOptions;

use zutility_be::{
    config::AppConfig,
    http,
    integrations::zcash::{
        validate_runtime_network_policy, validate_rpc_socket_policy,
        ZcashClient, ZcashRpcAdapter, ZcashRpcClient, ZingoClient,
    },
    observability, runtime,
};

#[tokio::main]
async fn main() -> Result<()> {
    observability::init_tracing();
    let config = AppConfig::from_env()?;
    let config = config.validate()?;
    validate_runtime_network_policy(&config)?;
    validate_rpc_socket_policy(&config)?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    let zcash_client: Option<Arc<dyn ZcashClient>> = match config.zcash_backend {
        zutility_be::config::ZcashBackend::Zingolib => {
            tracing::info!(
                indexer = %config.zingo_indexer_uri,
                wallet_dir = %config.zingo_wallet_dir,
                "initializing zingolib backend"
            );
            let chain_type = match config.zcash_network {
                zutility_be::config::ZcashNetwork::Testnet => zingolib::config::ChainType::Testnet,
                zutility_be::config::ZcashNetwork::Mainnet => zingolib::config::ChainType::Mainnet,
            };
            match ZingoClient::new(
                &config.zingo_indexer_uri,
                &config.zingo_wallet_dir,
                chain_type,
                config.zingo_sync_retries,
                config.zingo_sync_retry_delay_ms,
            )
            .await
            {
                Ok(client) => {
                    tracing::info!("zingolib client initialized and synced successfully");
                    Some(Arc::new(client))
                }
                Err(error) => {
                    tracing::error!(error = %error, "zingolib client initialization failed — starting without zcash client");
                    None
                }
            }
        }
        zutility_be::config::ZcashBackend::Rpc => {
            tracing::info!("initializing zcashd RPC backend");
            match ZcashRpcClient::from_app_config(&config) {
                Ok(rpc_client) => Some(Arc::new(ZcashRpcAdapter::new(rpc_client))),
                Err(error) => {
                    tracing::warn!(error = %error, "zcashd RPC client initialization failed — starting without zcash client");
                    None
                }
            }
        }
    };

    let state = http::build_state(&config, None, pool.clone());
    let state = match zcash_client {
        Some(client) => state.with_zcash_client(client),
        None => state,
    };

    runtime::start_background_workers(state.clone(), config.clone(), pool.clone());
    let app = http::build_router_from_state(state, true);
    let listener = tokio::net::TcpListener::bind(config.http_bind_addr).await?;
    tracing::info!(bind = %config.http_bind_addr, env = ?config.app_env, "backend http server started");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}
