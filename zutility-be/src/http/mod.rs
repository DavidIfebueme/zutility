use std::str::FromStr;

use axum::http::header::HeaderName;
use axum::{
    Router,
    routing::{get, post},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub mod auth;
pub mod error;
pub mod handlers;
pub mod types;

use crate::config::AppConfig;
use crate::integrations::rates::SharedRateCache;
use handlers::{
    HttpState, cancel_order, create_order, get_current_rate, get_order, list_utilities,
    stream_order, validate_utility_reference,
};

pub fn build_router(config: &AppConfig) -> Router {
    build_router_with_rate_cache_and_limits(config, None, true)
}

pub fn build_router_with_rate_cache(
    config: &AppConfig,
    rate_cache: Option<SharedRateCache>,
) -> Router {
    build_router_with_rate_cache_and_limits(config, rate_cache, true)
}

fn build_router_with_rate_cache_and_limits(
    config: &AppConfig,
    rate_cache: Option<SharedRateCache>,
    enable_rate_limits: bool,
) -> Router {

    let state = HttpState::new(
        config.order_token_hmac_secret.clone(),
        i64::from(config.order_expiry_minutes),
        i64::from(config.rate_lock_minutes),
    );
    let state = match rate_cache {
        Some(cache) => state.with_rate_cache(cache),
        None => state,
    };

    let router = Router::new()
        .route("/api/v1/orders/create", post(create_order))
        .route("/api/v1/orders/{order_id}", get(get_order))
        .route("/api/v1/orders/{order_id}/stream", get(stream_order))
        .route("/api/v1/orders/{order_id}/cancel", post(cancel_order))
        .route("/api/v1/rates/current", get(get_current_rate))
        .route("/api/v1/utilities", get(list_utilities))
        .route(
            "/api/v1/utilities/{slug}/validate",
            get(validate_utility_reference),
        )
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers([HeaderName::from_str("x-request-id").expect("valid header name")]),
        )
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(TraceLayer::new_for_http());

    if enable_rate_limits {
        let governor_config = GovernorConfigBuilder::default()
            .per_second(15)
            .burst_size(30)
            .use_headers()
            .finish()
            .expect("valid governor config");
        return router.layer(GovernorLayer::new(governor_config));
    }

    router
}

pub fn router() -> Router {
    let config = AppConfig {
        app_env: crate::config::AppEnv::Dev,
        http_bind_addr: "127.0.0.1:3001".parse().expect("valid bind address"),
        database_url: String::from("postgres://postgres:postgres@localhost:5432/zutility"),
        order_token_hmac_secret: secrecy::SecretString::from(String::from(
            "dev_order_token_secret",
        )),
        ip_hash_secret: secrecy::SecretString::from(String::from("dev_ip_secret")),
        vtpass_base_url: String::from("https://sandbox.vtpass.com/api"),
        vtpass_api_key: secrecy::SecretString::from(String::from("key")),
        vtpass_secret_key: secrecy::SecretString::from(String::from("secret")),
        zcash_rpc_mode: crate::config::ZcashRpcMode::Unix,
        zcash_rpc_socket_path: String::from("/var/run/zcashd/zcashd.sock"),
        zcash_rpc_url: String::from("http://127.0.0.1:18232"),
        zcash_rpc_user: secrecy::SecretString::from(String::from("rpc_user")),
        zcash_rpc_password: secrecy::SecretString::from(String::from("rpc_password")),
        zcash_network: crate::config::ZcashNetwork::Testnet,
        required_confs_transparent: 3,
        required_confs_shielded: 10,
        order_expiry_minutes: 30,
        rate_lock_minutes: 15,
        sweep_threshold_zec: rust_decimal::Decimal::new(5, 1),
        signing_service_url: String::from("http://10.0.0.2:8080"),
        signing_service_hmac_secret: secrecy::SecretString::from(String::from("hmac_secret")),
        rate_source_timeout_ms: 3000,
    };
    build_router_with_rate_cache_and_limits(&config, None, false)
}
