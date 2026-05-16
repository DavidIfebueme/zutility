use std::str::FromStr;

use axum::http::{header::HeaderName, HeaderValue};
use axum::{
    Router,
    middleware,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub mod auth;
pub mod auth_handlers;
pub mod docs;
pub mod error;
pub mod handlers;
pub mod mw;
pub mod types;
pub mod waitlist_handlers;

use crate::config::AppConfig;
use crate::integrations::rates::SharedRateCache;
use auth_handlers::{
    forgot_password, get_me, get_order_history, login, logout, refresh, register,
    resend_verification, reset_password, verify_email,
};
use docs::{docs_ui, openapi_json};
use handlers::{
    HttpState, alerts, cancel_order, create_order, get_current_rate, get_order, health_live,
    health_ready, list_utilities, list_utility_variations, metrics, stream_order,
    validate_utility_reference, webhook_inlomax, webhook_remita, webhook_vtpass,
};
use waitlist_handlers::{waitlist_join, waitlist_resend, waitlist_stats, waitlist_verify};

pub async fn build_router(config: &AppConfig) -> Result<Router, anyhow::Error> {
    let pool = PgPool::connect(&config.database_url).await?;
    let state = build_state(config, None, pool);
    Ok(build_router_from_state(state, true))
}

pub async fn build_router_with_rate_cache(
    config: &AppConfig,
    rate_cache: Option<SharedRateCache>,
    pool: PgPool,
) -> Router {
    let state = build_state(config, rate_cache, pool);
    build_router_from_state(state, true)
}

pub fn build_state(config: &AppConfig, rate_cache: Option<SharedRateCache>, pool: PgPool) -> HttpState {
    let state = HttpState::new(
        config.order_token_hmac_secret.clone(),
        i64::from(config.order_expiry_minutes),
        i64::from(config.rate_lock_minutes),
        pool,
        config.jwt_secret.clone(),
        i64::from(config.access_token_ttl_minutes),
        i64::from(config.refresh_token_ttl_hours),
        config.app_base_url.clone(),
    )
    .with_ops_context(config);

    match rate_cache {
        Some(cache) => state.with_rate_cache(cache),
        None => state,
    }
}

pub fn build_router_from_state(state: HttpState, enable_rate_limits: bool) -> Router {
    build_router_with_state_and_limits(state, enable_rate_limits)
}

fn build_router_with_state_and_limits(state: HttpState, enable_rate_limits: bool) -> Router {
    let jwt_secret_for_mw = state.jwt_secret.clone();
    let app_base_url = state.app_base_url.clone();

    let public_routes = Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/verify-email", post(verify_email))
        .route("/api/v1/auth/resend-verification", post(resend_verification))
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route("/api/v1/rates/current", get(get_current_rate))
        .route("/api/v1/utilities", get(list_utilities))
        .route(
            "/api/v1/utilities/{slug}/validate",
            get(validate_utility_reference),
        )
        .route(
            "/api/v1/utilities/{slug}/variations",
            get(list_utility_variations),
        )
        .route("/api/v1/webhooks/vtpass", post(webhook_vtpass))
        .route("/api/v1/webhooks/inlomax", post(webhook_inlomax))
        .route("/api/v1/webhooks/remita", post(webhook_remita))
        .route("/ops/health/live", get(health_live))
        .route("/ops/health/ready", get(health_ready))
        .route("/ops/openapi.json", get(openapi_json))
        .route("/ops/docs", get(docs_ui))
        .route("/ops/metrics", get(metrics))
        .route("/ops/alerts", get(alerts))
        .route("/api/v1/waitlist/join", post(waitlist_join))
        .route("/api/v1/waitlist/verify", post(waitlist_verify))
        .route("/api/v1/waitlist/resend", post(waitlist_resend))
        .route("/api/v1/waitlist/stats", get(waitlist_stats))
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route("/api/v1/orders/create", post(create_order))
        .route("/api/v1/orders/{order_id}", get(get_order))
        .route("/api/v1/orders/{order_id}/stream", get(stream_order))
        .route("/api/v1/orders/{order_id}/cancel", post(cancel_order))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/orders/history", get(get_order_history))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            jwt_secret_for_mw.clone(),
            mw::auth_middleware,
        ));

    let router = public_routes
        .merge(protected_routes)
        .layer({
            let cors = CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                    HeaderName::from_static("x-csrf-token"),
                    HeaderName::from_static("x-request-id"),
                ])
                .allow_credentials(true)
                .expose_headers([HeaderName::from_str("x-request-id").expect("valid header name")]);
            let allowed_origins = [
                app_base_url.clone(),
                app_base_url.replace("https://", "https://www."),
                app_base_url.replace("http://", "http://www."),
            ];
            let origins: Vec<HeaderValue> = allowed_origins
                .iter()
                .filter_map(|o| o.parse::<HeaderValue>().ok())
                .collect();
            cors.allow_origin(origins)
        })
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
            .per_second(30)
            .burst_size(200)
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
        remita_merchant_id: Some(String::from("demo_merchant")),
        remita_api_key: Some(secrecy::SecretString::from(String::from("remita_key"))),
        remita_service_type_id: Some(String::from("demo_service_type")),
        remita_base_url: Some(String::from("https://remitademo.net/remita/exapp/api/v1")),
        remita_webhook_secret: Some(secrecy::SecretString::from(String::from("remita_webhook"))),
        inlomax_api_key: None,
        inlomax_base_url: None,
        inlomax_webhook_secret: None,
        zcash_rpc_mode: crate::config::ZcashRpcMode::Unix,
        zcash_rpc_socket_path: String::from("/var/run/zcashd/zcashd.sock"),
        zcash_rpc_url: String::from("http://127.0.0.1:18232"),
        zcash_rpc_user: secrecy::SecretString::from(String::from("rpc_user")),
        zcash_rpc_password: secrecy::SecretString::from(String::from("rpc_password")),
        zcash_network: crate::config::ZcashNetwork::Testnet,
        zcash_backend: crate::config::ZcashBackend::Rpc,
        zingo_indexer_uri: String::from("https://testnet.zec.rocks"),
        zingo_wallet_dir: String::from("/tmp/zingo-wallet"),
        required_confs_transparent: 3,
        required_confs_shielded: 3,
        order_expiry_minutes: 30,
        rate_lock_minutes: 15,
        sweep_threshold_zec: rust_decimal::Decimal::new(5, 1),
        signing_service_url: String::from("http://10.0.0.2:8080"),
        signing_service_hmac_secret: secrecy::SecretString::from(String::from("hmac_secret")),
        rate_source_timeout_ms: 3000,
        jwt_secret: secrecy::SecretString::from(String::from("dev_jwt_secret_key_change_in_prod")),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_hours: 24,
        brevo_api_key: None,
        brevo_sender_email: None,
        brevo_sender_name: None,
        app_base_url: String::from("http://localhost:3000"),
    };

    let rate_cache = crate::integrations::rates::new_shared_rate_cache(
        crate::integrations::rates::CurrentRate {
            zec_ngn: rust_decimal::Decimal::new(150_000_0000, 4),
            zec_usd: rust_decimal::Decimal::new(100_0000, 4),
            usd_ngn: rust_decimal::Decimal::new(1500_0000, 4),
            usd_kes: rust_decimal::Decimal::new(129_0000, 4),
            usd_ghs: rust_decimal::Decimal::new(12_0000, 4),
            usd_zar: rust_decimal::Decimal::new(18_0000, 4),
            usd_egp: rust_decimal::Decimal::new(30_0000, 4),
            updated_at: chrono::Utc::now(),
        },
    );

    let pool = PgPool::connect_lazy(&config.database_url)
        .expect("valid pool for test router");

    let state = build_state(&config, Some(rate_cache), pool);
    build_router_from_state(state, false)
}
