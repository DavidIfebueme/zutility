use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::Response,
};
use chrono::{Duration, Utc};
use rand::{RngExt, distr::Alphanumeric};
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    domain::order::OrderStatus,
    http::auth,
    integrations::rates::{
        CurrentRate, SharedRateCache, default_current_rate, new_shared_rate_cache,
    },
    integrations::zcash::ZcashRpcClient,
    observability::{AlertState, ObservabilityState, ProbeStatus, ReadinessReport},
    ws::{self, WsHub, WsOrderEvent},
};

use super::{
    error::ApiError,
    types::{
        CancelOrderResponse, CreateOrderRequest, CreateOrderResponse, OrderRecord,
        OrderStatusResponse, OrderTokenQuery, RateResponse, UtilityItem, UtilityValidateQuery,
        UtilityValidateResponse,
    },
};

#[derive(Clone)]
pub struct HttpState {
    pub order_token_hmac_secret: secrecy::SecretString,
    pub order_expiry_minutes: i64,
    pub rate_lock_minutes: i64,
    pub orders: Arc<RwLock<HashMap<Uuid, OrderRecord>>>,
    pub ws_hub: WsHub,
    pub rate_cache: SharedRateCache,
    pub service_ref_velocity: Arc<RwLock<HashMap<String, Vec<chrono::DateTime<Utc>>>>>,
    pub observability: ObservabilityState,
    pub database_url: String,
    pub zcash_rpc_client: Option<ZcashRpcClient>,
    pub zcash_expected_chain: String,
}

impl HttpState {
    pub fn new(
        order_token_hmac_secret: secrecy::SecretString,
        order_expiry_minutes: i64,
        rate_lock_minutes: i64,
    ) -> Self {
        Self {
            order_token_hmac_secret,
            order_expiry_minutes,
            rate_lock_minutes,
            orders: Arc::new(RwLock::new(HashMap::new())),
            ws_hub: WsHub::new(),
            rate_cache: new_shared_rate_cache(default_current_rate()),
            service_ref_velocity: Arc::new(RwLock::new(HashMap::new())),
            observability: ObservabilityState::new(),
            database_url: String::new(),
            zcash_rpc_client: None,
            zcash_expected_chain: String::from("test"),
        }
    }

    pub fn with_rate_cache(mut self, rate_cache: SharedRateCache) -> Self {
        self.rate_cache = rate_cache;
        self
    }

    pub fn with_ops_context(mut self, config: &AppConfig) -> Self {
        self.database_url = config.database_url.clone();
        self.zcash_expected_chain = match config.zcash_network {
            crate::config::ZcashNetwork::Testnet => String::from("test"),
            crate::config::ZcashNetwork::Mainnet => String::from("main"),
        };
        self.zcash_rpc_client = ZcashRpcClient::from_app_config(config).ok();
        self.observability.jobs().mark_alive("http_server");
        self
    }
}

pub async fn create_order(
    State(state): State<HttpState>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<CreateOrderResponse>, ApiError> {
    validate_create_order_payload(&payload)?;
    enforce_service_ref_velocity(&state, &payload).await?;

    let order_id = Uuid::new_v4();
    state.observability.metrics().increment_order_creations();
    let order_access_token = generate_token(48);
    let access_token_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &order_access_token)
            .map_err(ApiError::internal)?;

    let rate = state.rate_cache.read().await.clone();
    let required_confirmations = if payload.zec_address_type == "shielded" {
        10
    } else {
        3
    };
    let zec_amount = Decimal::new(payload.amount_ngn, 0) / rate.zec_ngn;
    let expires_at = Utc::now() + Duration::minutes(state.order_expiry_minutes);
    let deposit_address = if payload.zec_address_type == "shielded" {
        String::from("ztestsapling1q3f4v8k6e4q7s9x2a5w6d8j9m3k2t7y8u6i5o4p3l2k1j0h9g8f7d6")
    } else {
        String::from("tmQ1Y8xQx5G4h5w6nJ4D31oQmRVVbYkA4W")
    };

    let record = OrderRecord {
        order_id,
        access_token_hash,
        utility_type: payload.utility_type.clone(),
        utility_slug: payload.utility_slug.clone(),
        service_ref: payload.service_ref.clone(),
        amount_ngn: payload.amount_ngn,
        zec_amount,
        deposit_address: deposit_address.clone(),
        status: OrderStatus::AwaitingPayment,
        confirmations: 0,
        required_confirmations,
        expires_at,
    };

    state.orders.write().await.insert(order_id, record);

    Ok(Json(CreateOrderResponse {
        order_id,
        order_access_token,
        deposit_address: deposit_address.clone(),
        zec_amount: zec_amount.round_dp(8).to_string(),
        expires_at,
        qr_data: format!("zcash:{deposit_address}?amount={}", zec_amount.round_dp(8)),
        required_confirmations,
    }))
}

async fn enforce_service_ref_velocity(
    state: &HttpState,
    payload: &CreateOrderRequest,
) -> Result<(), ApiError> {
    let now = Utc::now();
    let (window_minutes, max_requests) = match payload.utility_type.as_str() {
        "airtime" | "data" => (10_i64, 8_usize),
        "dstv" | "gotv" | "electricity" => (30_i64, 4_usize),
        _ => (10_i64, 5_usize),
    };

    let key = format!(
        "{}:{}:{}",
        payload.utility_type.trim().to_ascii_lowercase(),
        payload.utility_slug.trim().to_ascii_lowercase(),
        payload.service_ref.trim().to_ascii_lowercase()
    );

    let mut tracker = state.service_ref_velocity.write().await;
    let entries = tracker.entry(key).or_default();
    let cutoff = now - Duration::minutes(window_minutes);
    entries.retain(|timestamp| *timestamp >= cutoff);

    if entries.len() >= max_requests {
        return Err(ApiError::too_many_requests(
            "service_ref velocity limit exceeded",
        ));
    }

    entries.push(now);
    Ok(())
}

pub async fn get_order(
    Path(order_id): Path<Uuid>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
) -> Result<Json<OrderStatusResponse>, ApiError> {
    let order = authorize_order_access(&state, order_id, &query.token).await?;

    Ok(Json(OrderStatusResponse {
        order_id: order.order_id,
        status: order.status,
        confirmations: order.confirmations,
        required_confirmations: order.required_confirmations,
        total_received: None,
        utility_type: order.utility_type,
        utility_slug: order.utility_slug,
        service_ref: order.service_ref,
        amount_ngn: order.amount_ngn,
        zec_amount: order.zec_amount.round_dp(8).to_string(),
        expires_at: order.expires_at,
        completed_at: None,
        delivery_token: None,
    }))
}

pub async fn cancel_order(
    Path(order_id): Path<Uuid>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
) -> Result<Json<CancelOrderResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    if !auth::verify_order_token_hash(
        &state.order_token_hmac_secret,
        &query.token,
        &order.access_token_hash,
    ) {
        return Err(ApiError::forbidden("invalid order token"));
    }

    if order.status != OrderStatus::AwaitingPayment {
        return Err(ApiError::conflict(
            "order can only be cancelled in awaiting_payment",
        ));
    }

    order.status = OrderStatus::Cancelled;
    Ok(Json(CancelOrderResponse { success: true }))
}

pub async fn stream_order(
    Path(order_id): Path<Uuid>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let order = authorize_order_access(&state, order_id, &query.token).await?;
    let hub = state.ws_hub.clone();
    let initial_event = map_status_to_event(&order);

    Ok(ws.on_upgrade(move |socket| ws::serve_connection(hub, order_id, socket, initial_event)))
}

pub async fn get_current_rate(
    State(state): State<HttpState>,
) -> Result<Json<RateResponse>, ApiError> {
    let CurrentRate {
        zec_ngn,
        zec_usd,
        updated_at,
        ..
    } = state.rate_cache.read().await.clone();
    let valid_until = updated_at + Duration::minutes(state.rate_lock_minutes);

    Ok(Json(RateResponse {
        zec_ngn: zec_ngn.round_dp(4).to_string(),
        zec_usd: zec_usd.round_dp(4).to_string(),
        updated_at,
        valid_until,
    }))
}

pub async fn health_live() -> StatusCode {
    StatusCode::OK
}

pub async fn health_ready(State(state): State<HttpState>) -> Json<ReadinessReport> {
    Json(build_readiness_report(&state).await)
}

pub async fn metrics(State(state): State<HttpState>) -> Result<String, ApiError> {
    Ok(state.observability.metrics().render_prometheus().await)
}

pub async fn alerts(State(state): State<HttpState>) -> Json<Vec<AlertState>> {
    let readiness = build_readiness_report(&state).await;
    let rate_last_updated = state.rate_cache.read().await.updated_at;
    let alerts = state
        .observability
        .evaluate_alerts(&readiness, rate_last_updated)
        .await;
    Json(alerts)
}

pub async fn list_utilities() -> Result<Json<Vec<UtilityItem>>, ApiError> {
    Ok(Json(vec![
        UtilityItem {
            slug: String::from("mtn"),
            utility_type: String::from("airtime"),
            name: String::from("MTN Airtime"),
        },
        UtilityItem {
            slug: String::from("airtel"),
            utility_type: String::from("airtime"),
            name: String::from("Airtel Airtime"),
        },
        UtilityItem {
            slug: String::from("glo"),
            utility_type: String::from("airtime"),
            name: String::from("Glo Airtime"),
        },
        UtilityItem {
            slug: String::from("9mobile"),
            utility_type: String::from("airtime"),
            name: String::from("9mobile Airtime"),
        },
        UtilityItem {
            slug: String::from("dstv"),
            utility_type: String::from("dstv"),
            name: String::from("DSTV"),
        },
        UtilityItem {
            slug: String::from("gotv"),
            utility_type: String::from("gotv"),
            name: String::from("GOtv"),
        },
        UtilityItem {
            slug: String::from("phcn"),
            utility_type: String::from("electricity"),
            name: String::from("Electricity"),
        },
    ]))
}

pub async fn validate_utility_reference(
    Path(slug): Path<String>,
    Query(query): Query<UtilityValidateQuery>,
) -> Result<Json<UtilityValidateResponse>, ApiError> {
    if query.reference.trim().is_empty() {
        return Err(ApiError::bad_request("ref is required"));
    }

    let valid = matches!(
        slug.as_str(),
        "mtn" | "airtel" | "glo" | "9mobile" | "dstv" | "gotv" | "phcn"
    );

    Ok(Json(UtilityValidateResponse {
        valid,
        customer_name: if valid {
            Some(String::from("Validated Customer"))
        } else {
            None
        },
    }))
}

fn validate_create_order_payload(payload: &CreateOrderRequest) -> Result<(), ApiError> {
    if payload.utility_type.trim().is_empty() {
        return Err(ApiError::bad_request("utility_type is required"));
    }
    if payload.utility_slug.trim().is_empty() {
        return Err(ApiError::bad_request("utility_slug is required"));
    }
    if payload.service_ref.trim().is_empty() {
        return Err(ApiError::bad_request("service_ref is required"));
    }
    if payload.amount_ngn <= 0 {
        return Err(ApiError::bad_request("amount_ngn must be greater than 0"));
    }
    if !matches!(
        payload.zec_address_type.as_str(),
        "shielded" | "transparent"
    ) {
        return Err(ApiError::bad_request(
            "zec_address_type must be shielded or transparent",
        ));
    }
    Ok(())
}

fn generate_token(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

async fn build_readiness_report(state: &HttpState) -> ReadinessReport {
    let db_probe = match probe_db_connectivity(&state.database_url).await {
        Ok(true) => ProbeStatus {
            healthy: true,
            detail: String::from("database connectivity ok"),
        },
        Ok(false) => ProbeStatus {
            healthy: false,
            detail: String::from("database connectivity unavailable"),
        },
        Err(error) => ProbeStatus {
            healthy: false,
            detail: format!("database probe failed: {error}"),
        },
    };

    let zcash_probe = match &state.zcash_rpc_client {
        Some(client) => match client.get_blockchain_info().await {
            Ok(info) => {
                state
                    .observability
                    .metrics()
                    .set_zcash_sync_lag_blocks(info.headers.saturating_sub(info.blocks));
                ProbeStatus {
                    healthy: info.chain == state.zcash_expected_chain,
                    detail: format!(
                        "chain={} verification_progress={:.4}",
                        info.chain, info.verification_progress
                    ),
                }
            }
            Err(error) => ProbeStatus {
                healthy: false,
                detail: format!("zcash rpc probe failed: {error}"),
            },
        },
        None => ProbeStatus {
            healthy: false,
            detail: String::from("zcash rpc client unavailable"),
        },
    };

    let rate_updated_at = state.rate_cache.read().await.updated_at;
    let rate_age_seconds = (Utc::now() - rate_updated_at).num_seconds();
    let rate_probe = ProbeStatus {
        healthy: rate_age_seconds <= 300,
        detail: format!("rate age seconds={rate_age_seconds}"),
    };

    let jobs_registry = state.observability.jobs();
    let stale_jobs = jobs_registry.stale_jobs(180);
    let jobs_probe = ProbeStatus {
        healthy: jobs_registry.has_any_heartbeat() && stale_jobs.is_empty(),
        detail: if stale_jobs.is_empty() {
            String::from("job heartbeats healthy")
        } else {
            format!("stale jobs: {}", stale_jobs.join(","))
        },
    };

    let ready = db_probe.healthy && zcash_probe.healthy && rate_probe.healthy && jobs_probe.healthy;

    ReadinessReport {
        db: db_probe,
        zcash: zcash_probe,
        rate_cache: rate_probe,
        jobs: jobs_probe,
        ready,
    }
}

async fn probe_db_connectivity(database_url: &str) -> anyhow::Result<bool> {
    if database_url.trim().is_empty() {
        return Ok(false);
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect(database_url)
        .await;

    match pool {
        Ok(pool) => {
            let ping = sqlx::query_scalar::<_, i64>("SELECT 1")
                .fetch_one(&pool)
                .await;
            pool.close().await;
            Ok(ping.is_ok())
        }
        Err(_) => Ok(false),
    }
}

async fn authorize_order_access(
    state: &HttpState,
    order_id: Uuid,
    token: &str,
) -> Result<OrderRecord, ApiError> {
    let order = state
        .orders
        .read()
        .await
        .get(&order_id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    if !auth::verify_order_token_hash(
        &state.order_token_hmac_secret,
        token,
        &order.access_token_hash,
    ) {
        return Err(ApiError::forbidden("invalid order token"));
    }

    Ok(order)
}

fn map_status_to_event(order: &OrderRecord) -> Option<WsOrderEvent> {
    match order.status {
        OrderStatus::PaymentDetected => Some(WsOrderEvent::PaymentDetected {
            confirmations: order.confirmations,
            required: order.required_confirmations,
        }),
        OrderStatus::PaymentConfirmed => Some(WsOrderEvent::PaymentConfirmed {
            confirmations: order.confirmations,
        }),
        OrderStatus::UtilityDispatching => Some(WsOrderEvent::Dispatching),
        OrderStatus::Completed => Some(WsOrderEvent::Completed {
            delivery_token: None,
            reference: order.order_id.to_string(),
        }),
        OrderStatus::Expired => Some(WsOrderEvent::Expired),
        OrderStatus::Failed => Some(WsOrderEvent::Failed {
            reason: String::from("order_failed"),
        }),
        OrderStatus::AwaitingPayment | OrderStatus::FlaggedForReview | OrderStatus::Cancelled => {
            None
        }
    }
}
