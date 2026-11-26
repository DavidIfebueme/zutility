use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    Json,
    extract::{ConnectInfo, Extension, Path, Query, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::Response,
};
use chrono::{Duration, Utc};
use rand::{RngExt, distr::Alphanumeric};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    config::{AppConfig, ZcashNetwork},
    db::{self, CreateOrderInput, OrderRow},
    domain::money::{CurrencyCode, Money},
    domain::order::{OrderStatus, OrderStatusTransition},
    domain::pricing,
    domain::service_ref,
    http::auth,
    integrations::provider_dispatcher::ProviderDispatcher,
    integrations::rates::{
        CurrentRate, FX_QUOTES, SharedRateCache, default_current_rate, new_shared_rate_cache,
    },
    integrations::remita::RemitaClient,
    integrations::utility_provider::{
        ProviderError, ProviderErrorKind, ProviderTxnStatus, UtilityProvider, ValidateRefRequest,
    },
    integrations::vtpass::VtpassClient,
    integrations::zcash::ZcashClient,
    observability::{AlertState, ObservabilityState, ProbeStatus, ReadinessReport},
    ws::{self, WsHub, WsOrderEvent},
};

use super::{
    error::ApiError,
    types::{
        CancelOrderResponse, CreateOrderRequest, CreateOrderResponse, CurrencyQuery, DisplayMoney,
        FxQuoteResponse, MoneyDto, OrderAccessTokenResponse, OrderStatusResponse, OrderTokenQuery,
        RateResponse, UtilityValidateQuery, UtilityValidateResponse, UtilityVariationItem,
    },
};

pub(crate) const DEFAULT_COUNTRY_ISO2: &str = "NG";

#[derive(Clone)]
pub struct HttpState {
    pub order_token_hmac_secret: secrecy::SecretString,
    pub order_expiry_minutes: i64,
    pub rate_lock_minutes: i64,
    pub pool: PgPool,
    pub ws_hub: WsHub,
    pub rate_cache: SharedRateCache,
    pub service_ref_velocity: Arc<RwLock<HashMap<String, Vec<chrono::DateTime<Utc>>>>>,
    pub observability: ObservabilityState,
    pub zcash_client: Option<Arc<dyn ZcashClient>>,
    pub zcash_network: ZcashNetwork,
    pub zcash_expected_chain: String,
    pub required_confs_transparent: u16,
    pub required_confs_shielded: u16,
    pub catalogue_max_age_minutes: u32,
    pub provider_dispatcher: Option<ProviderDispatcher>,
    pub jwt_secret: secrecy::SecretString,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_hours: i64,
    pub email_client: Option<crate::integrations::email::EmailClient>,
    pub app_base_url: String,
    pub frontend_url: Option<String>,
    pub cookie_domain: Option<String>,
    pub admin_secret: Option<secrecy::SecretString>,
}

impl HttpState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        order_token_hmac_secret: secrecy::SecretString,
        order_expiry_minutes: i64,
        rate_lock_minutes: i64,
        pool: PgPool,
        jwt_secret: secrecy::SecretString,
        access_token_ttl_minutes: i64,
        refresh_token_ttl_hours: i64,
        app_base_url: String,
    ) -> Self {
        Self {
            order_token_hmac_secret,
            order_expiry_minutes,
            rate_lock_minutes,
            pool,
            ws_hub: WsHub::new(),
            rate_cache: new_shared_rate_cache(default_current_rate()),
            service_ref_velocity: Arc::new(RwLock::new(HashMap::new())),
            observability: ObservabilityState::new(),
            zcash_client: None,
            zcash_network: ZcashNetwork::Testnet,
            zcash_expected_chain: String::from("test"),
            required_confs_transparent: 2,
            required_confs_shielded: 2,
            catalogue_max_age_minutes: 1440,
            provider_dispatcher: None,
            jwt_secret,
            access_token_ttl_minutes,
            refresh_token_ttl_hours,
            email_client: None,
            app_base_url,
            frontend_url: None,
            cookie_domain: None,
            admin_secret: None,
        }
    }

    pub fn with_rate_cache(mut self, rate_cache: SharedRateCache) -> Self {
        self.rate_cache = rate_cache;
        self
    }

    pub fn with_ops_context(mut self, config: &AppConfig) -> Self {
        self.zcash_network = config.zcash_network.clone();
        self.zcash_expected_chain = config.zcash_network.expected_chain().to_owned();
        self.required_confs_transparent = config.required_confs_transparent;
        self.required_confs_shielded = config.required_confs_shielded;
        self.catalogue_max_age_minutes = config.catalogue_max_age_minutes;

        let vtpass = VtpassClient::from_config(config).ok();
        let remita = RemitaClient::from_config(config).ok();
        let remita_collections =
            crate::integrations::remita_collections::RemitaCollectionsClient::from_config(config)
                .ok();
        let inlomax = crate::integrations::inlomax::InlomaxClient::from_config(config).ok();
        let dtone = match crate::integrations::dtone::DtoneClient::from_config(config) {
            Ok(client) => {
                tracing::info!("dtone client initialized successfully");
                Some(client)
            }
            Err(error) => {
                tracing::warn!(error = %error, "dtone client not configured — multi-country utilities will be unavailable");
                None
            }
        };
        if let Some(vtpass) = vtpass {
            self.provider_dispatcher = Some(
                ProviderDispatcher::new(vtpass, remita, remita_collections, inlomax, dtone)
                    .with_metrics(self.observability.metrics()),
            );
        }

        if let (Some(api_key), Some(sender_email), Some(sender_name)) = (
            &config.brevo_api_key,
            &config.brevo_sender_email,
            &config.brevo_sender_name,
        ) {
            self.email_client = Some(crate::integrations::email::EmailClient::new(
                api_key,
                sender_email,
                sender_name,
            ));
            tracing::info!("brevo email client configured");
        } else {
            tracing::warn!(
                "brevo email client not configured — verification emails will be logged instead"
            );
        }

        if config.admin_secret.is_some() {
            tracing::info!("admin secret configured — admin endpoints enabled");
        } else {
            tracing::warn!("admin secret not configured — admin endpoints disabled");
        }
        self.admin_secret = config.admin_secret.clone();
        self.frontend_url = config.frontend_url.clone();
        self.cookie_domain = config.cookie_domain.clone();

        self
    }

    pub fn with_zcash_client(mut self, client: Arc<dyn ZcashClient>) -> Self {
        self.zcash_client = Some(client);
        self
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/create",
    request_body = CreateOrderRequest,
    responses(
        (status = 200, description = "Order created", body = CreateOrderResponse),
        (status = 400, description = "Validation error", body = super::error::ErrorEnvelope),
        (status = 409, description = "An open order already exists for this RRR", body = super::error::ErrorEnvelope),
        (status = 429, description = "Rate limit exceeded", body = super::error::ErrorEnvelope),
        (status = 503, description = "Remita unavailable", body = super::error::ErrorEnvelope),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn create_order(
    State(state): State<HttpState>,
    Extension(auth_user): Extension<crate::http::mw::AuthenticatedUser>,
    Json(mut payload): Json<CreateOrderRequest>,
) -> Result<Json<CreateOrderResponse>, ApiError> {
    validate_create_order_payload(&payload)?;

    if db::find_user_by_id(&state.pool, auth_user.user_id)
        .await
        .map_err(internal_err)?
        .is_none()
    {
        return Err(ApiError::unauthorized("account is no longer active"));
    }

    let resolved = resolve_product(&state, &payload).await?;
    payload.utility_type = resolved.product.utility_type.clone();
    payload.utility_slug = resolved.product.slug.clone();

    payload.service_ref = service_ref::normalize(
        &resolved.product.country_iso2,
        &resolved.product.utility_type,
        &payload.service_ref,
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))?;

    enforce_service_ref_velocity(&state, &payload).await?;

    if payload.utility_type == "school_fees"
        && db::active_school_fee_order_exists(&state.pool, payload.service_ref.trim())
            .await
            .map_err(internal_err)?
    {
        return Err(ApiError::conflict(
            "an open order already exists for this RRR",
        ));
    }

    let terms = resolved.product.terms();
    let settlement = if payload.utility_type == "school_fees" {
        resolve_school_fee_settlement(&state, &payload).await?
    } else {
        let settlement = request_settlement(&payload)?;
        pricing::validate_settlement(
            &terms,
            settlement,
            resolved
                .variation
                .as_ref()
                .map(|variation| variation.amount),
        )
        .map_err(pricing_error)?;
        settlement
    };

    let priced = pricing::price(&terms, settlement).map_err(pricing_error)?;

    state.observability.metrics().increment_order_creations();
    let order_access_token = generate_token(48);
    let access_token_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &order_access_token)
            .map_err(internal_err)?;

    let rate = state.rate_cache.read().await.clone();
    let required_confirmations = if payload.zec_address_type == "shielded" {
        state.required_confs_shielded
    } else {
        state.required_confs_transparent
    };
    if rate.zec_ngn <= Decimal::ZERO {
        return Err(ApiError::internal(
            "rate data unavailable — please try again in a moment",
        ));
    }

    if Utc::now() - rate.updated_at > Duration::minutes(state.rate_lock_minutes) {
        return Err(ApiError::service_unavailable(
            "the exchange rate is stale, please try again in a moment",
        ));
    }
    let zec_amount = Decimal::new(priced.amount_ngn, 0) / rate.zec_ngn;
    let expires_at = Utc::now() + Duration::minutes(state.order_expiry_minutes);

    let rate_snapshot_id = get_or_create_rate_snapshot(&state.pool, &rate)
        .await
        .map_err(internal_err)?;

    let input = CreateOrderInput {
        access_token_hash,
        utility_type: payload.utility_type.clone(),
        utility_slug: payload.utility_slug.clone(),
        service_ref: payload.service_ref.trim().to_owned(),
        amount_ngn: priced.amount_ngn,
        country_iso2: resolved.product.country_iso2.clone(),
        product_id: Some(resolved.product.id),
        settlement,
        cost_amount: priced.cost_amount,
        inputs: payload.inputs.clone(),
        fulfilment_kind: Some(resolved.product.fulfilment_kind.clone()),
        address_type: payload.zec_address_type.clone(),
        zec_amount,
        zec_rate_id: rate_snapshot_id,
        required_confs: i32::from(required_confirmations),
        expires_at,
        ip_hash: None,
        metadata: serde_json::json!({
            "utility_slug": payload.utility_slug,
            "variation_code": payload.variation_code,
            "meter_type": payload.meter_type,
            "country_iso2": resolved.product.country_iso2,
            "product_id": resolved.product.id,
        }),
        variation_code: payload.variation_code.clone(),
    };

    let network = state.zcash_network.as_str();
    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;
    let (order_id, deposit_address) =
        db::insert_order_with_claimed_address(&mut tx, &input, network)
            .await
            .map_err(|error| {
                if is_school_fee_rrr_conflict(&error) {
                    ApiError::conflict("an open order already exists for this RRR")
                } else {
                    internal_err(error)
                }
            })?;
    db::set_order_user_id_tx(&mut tx, order_id, auth_user.user_id)
        .await
        .map_err(internal_err)?;
    tx.commit().await.map_err(internal_err)?;

    Ok(Json(CreateOrderResponse {
        order_id,
        order_access_token,
        deposit_address: deposit_address.clone(),
        zec_amount: zec_amount.round_dp(8).to_string(),
        expires_at,
        qr_data: format!("zcash:{deposit_address}?amount={}", zec_amount.round_dp(8)),
        required_confirmations,
        address_type: payload.zec_address_type.clone(),
        utility_slug: payload.utility_slug.clone(),
    }))
}

pub(crate) struct ResolvedProduct {
    pub product: db::ProductRow,
    pub variation: Option<db::ProductVariationRow>,
}

pub(crate) async fn resolve_product(
    state: &HttpState,
    payload: &CreateOrderRequest,
) -> Result<ResolvedProduct, ApiError> {
    let product = match payload.product_id {
        Some(product_id) => db::find_product_by_id(&state.pool, product_id)
            .await
            .map_err(internal_err)?,
        None => {
            let country = payload
                .country_iso2
                .as_deref()
                .unwrap_or(DEFAULT_COUNTRY_ISO2);
            db::find_product_by_country_slug(&state.pool, country, payload.utility_slug.trim())
                .await
                .map_err(internal_err)?
        }
    };

    let Some(product) = product else {
        return Err(ApiError::bad_request("product_unavailable"));
    };

    if !product.enabled || !product.country_enabled {
        return Err(ApiError::bad_request("product_unavailable"));
    }

    if product.provider == crate::integrations::dtone::PROVIDER_NAME
        && product.country_iso2 != DEFAULT_COUNTRY_ISO2
    {
        let max_age = i64::from(state.catalogue_max_age_minutes);
        if Utc::now() - product.synced_at > Duration::minutes(max_age) {
            return Err(ApiError::service_unavailable(
                "the product price list is stale, try again shortly",
            ));
        }
    }

    let variation = match payload.variation_code.as_deref() {
        Some(code) => {
            let variation = db::find_product_variation(&state.pool, product.id, code)
                .await
                .map_err(internal_err)?;
            match variation {
                Some(variation) if variation.enabled => Some(variation),
                _ => {
                    return Err(ApiError::bad_request("that variation is not available"));
                }
            }
        }
        None => None,
    };

    Ok(ResolvedProduct { product, variation })
}

fn request_settlement(payload: &CreateOrderRequest) -> Result<Money, ApiError> {
    if let Some(settlement) = &payload.settlement {
        let currency = CurrencyCode::from_iso(&settlement.currency)
            .ok_or_else(|| ApiError::bad_request("unsupported settlement currency"))?;
        return Money::from_major(currency, &settlement.amount)
            .map_err(|error| ApiError::bad_request(error.to_string()));
    }

    let amount_ngn = payload
        .amount_ngn
        .ok_or_else(|| ApiError::bad_request("amount_ngn or settlement is required"))?;

    Money::from_ngn_whole(amount_ngn).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn pricing_error(error: pricing::PricingError) -> ApiError {
    match error {
        pricing::PricingError::ProductUnavailable => ApiError::bad_request("product_unavailable"),
        pricing::PricingError::CatalogueStale => {
            ApiError::service_unavailable("the product price list is stale, try again shortly")
        }
        other => ApiError::bad_request(other.to_string()),
    }
}

async fn resolve_school_fee_settlement(
    state: &HttpState,
    payload: &CreateOrderRequest,
) -> Result<Money, ApiError> {
    let reference = payload.service_ref.trim();
    if !is_rrr(reference) {
        return Err(ApiError::bad_request(
            "school fees need a 12 digit Remita RRR",
        ));
    }

    let dispatcher = state
        .provider_dispatcher
        .as_ref()
        .ok_or_else(|| ApiError::internal("school fees are not configured"))?;

    let request = ValidateRefRequest {
        service_id: payload.utility_slug.clone(),
        billers_code: reference.to_owned(),
        meter_type: None,
    };

    let details = dispatcher
        .validate_reference("school_fees", &request)
        .await
        .map_err(remita_lookup_error)?;

    if !details.is_valid {
        return Err(ApiError::bad_request("that RRR is not payable"));
    }

    let looked_up = details
        .settlement
        .and_then(|settlement| settlement.to_ngn_whole().ok())
        .ok_or_else(|| ApiError::bad_request("could not read the amount on that RRR"))?;

    if looked_up <= 0 {
        return Err(ApiError::bad_request("that RRR has no amount"));
    }

    if looked_up > pricing::MAX_ORDER_AMOUNT_NGN {
        return Err(ApiError::bad_request(format!(
            "school fees above {} naira are not supported",
            pricing::MAX_ORDER_AMOUNT_NGN
        )));
    }

    if let Some(client_amount) = payload.amount_ngn
        && client_amount != looked_up
    {
        tracing::info!(
            client_amount,
            looked_up_amount = looked_up,
            "school fees amount taken from the RRR lookup"
        );
    }

    Money::from_ngn_whole(looked_up).map_err(internal_err)
}

fn velocity_key(utility_type: &str, utility_slug: &str, service_ref: &str) -> String {
    let part = |value: &str, max: usize| {
        value
            .trim()
            .chars()
            .take(max)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    format!(
        "{}:{}:{}",
        part(utility_type, 32),
        part(utility_slug, 32),
        part(service_ref, 64)
    )
}

async fn enforce_service_ref_velocity(
    state: &HttpState,
    payload: &CreateOrderRequest,
) -> Result<(), ApiError> {
    let now = Utc::now();
    let (window_minutes, max_requests) = match payload.utility_type.as_str() {
        "airtime" | "data" => (10_i64, 8_usize),
        "dstv" | "gotv" | "startimes" | "showmax" | "electricity" => (30_i64, 4_usize),
        "school_fees" | "waec" | "jamb" => (60_i64, 2_usize),
        _ => (10_i64, 5_usize),
    };

    let key = velocity_key(
        &payload.utility_type,
        &payload.utility_slug,
        &payload.service_ref,
    );

    let mut tracker = state.service_ref_velocity.write().await;
    if tracker.len() > 10_000 {
        let cutoff = now - Duration::minutes(60);
        tracker.retain(|_, timestamps| timestamps.iter().any(|timestamp| *timestamp >= cutoff));
    }
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

#[utoipa::path(
    get,
    path = "/api/v1/orders/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order id"),
        ("token" = String, Query, description = "Order access token")
    ),
    responses(
        (status = 200, description = "Order status", body = OrderStatusResponse),
        (status = 403, description = "Invalid order token", body = super::error::ErrorEnvelope),
        (status = 404, description = "Order not found", body = super::error::ErrorEnvelope),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn get_order(
    Path(order_id): Path<Uuid>,
    Extension(auth_user): Extension<crate::http::mw::AuthenticatedUser>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
) -> Result<Json<OrderStatusResponse>, ApiError> {
    let order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    let is_owner = order_matches_user(&order, &auth_user);
    let token_valid = auth::verify_order_token_hash(
        &state.order_token_hmac_secret,
        &query.token,
        &order.access_token_hash,
    );

    if !is_owner && !token_valid {
        return Err(ApiError::forbidden("access denied"));
    }

    let status = order.status().map_err(internal_err)?;

    let display = display_money(
        &state,
        Some(&auth_user),
        query.currency.as_deref(),
        Money::from_ngn_whole(order.amount_ngn).ok(),
    )
    .await;

    Ok(Json(OrderStatusResponse {
        order_id: order.id,
        status,
        confirmations: u16::try_from(order.confirmations).unwrap_or(u16::MAX),
        required_confirmations: u16::try_from(order.required_confs).unwrap_or(u16::MAX),
        total_received: order
            .total_received
            .map(|value| value.round_dp(8).to_string()),
        utility_type: order.utility_type,
        utility_slug: order.utility_slug,
        service_ref: order.service_ref,
        amount_ngn: order.amount_ngn,
        zec_amount: order.zec_amount.round_dp(8).to_string(),
        deposit_address: order.deposit_address.clone(),
        qr_data: format!(
            "zcash:{}?amount={}",
            order.deposit_address,
            order.zec_amount.round_dp(8)
        ),
        address_type: order.address_type.clone(),
        expires_at: order.expires_at,
        completed_at: order.completed_at,
        delivery_token: order.delivery_token,
        failure_code: order.failure_code,
        failure_message: order.failure_message,
        settlement: order.settlement.map(MoneyDto::from_money),
        fulfilment_kind: order.fulfilment_kind.unwrap_or_default(),
        display,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/{order_id}/cancel",
    params(
        ("order_id" = Uuid, Path, description = "Order id"),
        ("token" = String, Query, description = "Order access token")
    ),
    responses(
        (status = 200, description = "Order cancelled", body = CancelOrderResponse),
        (status = 403, description = "Invalid order token", body = super::error::ErrorEnvelope),
        (status = 404, description = "Order not found", body = super::error::ErrorEnvelope),
        (status = 409, description = "Invalid status for cancel", body = super::error::ErrorEnvelope),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn cancel_order(
    Path(order_id): Path<Uuid>,
    Extension(auth_user): Extension<crate::http::mw::AuthenticatedUser>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
) -> Result<Json<CancelOrderResponse>, ApiError> {
    let order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    let is_owner = order_matches_user(&order, &auth_user);
    let token_valid = auth::verify_order_token_hash(
        &state.order_token_hmac_secret,
        &query.token,
        &order.access_token_hash,
    );

    if !is_owner && !token_valid {
        return Err(ApiError::forbidden("access denied"));
    }

    let status = order.status().map_err(internal_err)?;
    if !status.can_transition_to(OrderStatus::Cancelled) {
        return Err(ApiError::conflict(
            "order can only be cancelled in awaiting_payment",
        ));
    }

    let cancelled = db::update_order_status_cas(
        &state.pool,
        order_id,
        status.as_db(),
        OrderStatus::Cancelled.as_db(),
        "order_cancelled",
        serde_json::json!({}),
    )
    .await
    .map_err(internal_err)?;

    db::cancel_gift_by_order_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?;

    if cancelled {
        let _ = state
            .ws_hub
            .broadcast_event(order_id, &WsOrderEvent::Cancelled)
            .await;
    }

    Ok(Json(CancelOrderResponse { success: true }))
}

#[utoipa::path(
    post,
    path = "/api/v1/orders/{order_id}/access-token",
    params(("order_id" = Uuid, Path, description = "Order id")),
    responses(
        (status = 200, description = "Fresh order access token", body = OrderAccessTokenResponse),
        (status = 403, description = "Not the order owner", body = super::error::ErrorEnvelope),
        (status = 404, description = "Order not found", body = super::error::ErrorEnvelope)
    )
)]
pub async fn rotate_order_access_token(
    Path(order_id): Path<Uuid>,
    Extension(auth_user): Extension<crate::http::mw::AuthenticatedUser>,
    State(state): State<HttpState>,
) -> Result<Json<OrderAccessTokenResponse>, ApiError> {
    let order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    if !order_matches_user(&order, &auth_user) {
        return Err(ApiError::forbidden("access denied"));
    }

    let order_access_token = generate_token(48);
    let access_token_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &order_access_token)
            .map_err(internal_err)?;

    db::set_order_access_token_hash(&state.pool, order_id, &access_token_hash)
        .await
        .map_err(internal_err)?;

    Ok(Json(OrderAccessTokenResponse { order_access_token }))
}

pub async fn stream_order(
    Path(order_id): Path<Uuid>,
    Extension(auth_user): Extension<crate::http::mw::AuthenticatedUser>,
    Query(query): Query<OrderTokenQuery>,
    State(state): State<HttpState>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("order not found"))?;

    let is_owner = order_matches_user(&order, &auth_user);
    let token_valid = auth::verify_order_token_hash(
        &state.order_token_hmac_secret,
        &query.token,
        &order.access_token_hash,
    );

    if !is_owner && !token_valid {
        return Err(ApiError::forbidden("access denied"));
    }

    let hub = state.ws_hub.clone();
    let initial_event = map_status_to_event(&order);

    Ok(ws.on_upgrade(move |socket| ws::serve_connection(hub, order_id, socket, initial_event)))
}

#[utoipa::path(
    get,
    path = "/api/v1/rates/current",
    responses(
        (status = 200, description = "Current rate", body = RateResponse),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn get_current_rate(
    Query(query): Query<CurrencyQuery>,
    State(state): State<HttpState>,
) -> Result<Json<RateResponse>, ApiError> {
    let CurrentRate {
        zec_ngn,
        zec_usd,
        usd_ngn,
        usd_kes,
        usd_ghs,
        usd_zar,
        usd_egp,
        updated_at,
    } = state.rate_cache.read().await.clone();
    let valid_until = updated_at + Duration::minutes(state.rate_lock_minutes);

    let fx = match db::list_fx_rates(&state.pool).await {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| FX_QUOTES.iter().any(|code| code.iso() == row.quote))
            .map(|row| FxQuoteResponse {
                quote: row.quote,
                per_usd: row.per_usd.to_string(),
                fetched_at: row.fetched_at,
            })
            .collect(),
        Err(error) => {
            tracing::warn!(error = %error, "fx rate list read failed");
            Vec::new()
        }
    };

    let display = display_money(
        &state,
        None,
        query.currency.as_deref(),
        Money::new_rounded(CurrencyCode::Ngn, usd_ngn).ok(),
    )
    .await;

    Ok(Json(RateResponse {
        zec_ngn: zec_ngn.round_dp(4).to_string(),
        zec_usd: zec_usd.round_dp(4).to_string(),
        usd_ngn: usd_ngn.round_dp(4).to_string(),
        usd_kes: usd_kes.round_dp(4).to_string(),
        usd_ghs: usd_ghs.round_dp(4).to_string(),
        usd_zar: usd_zar.round_dp(4).to_string(),
        usd_egp: usd_egp.round_dp(4).to_string(),
        updated_at,
        valid_until,
        fx,
        display,
    }))
}

#[utoipa::path(
    get,
    path = "/ops/health/live",
    responses((status = 200, description = "Liveness probe"))
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/utilities/{slug}/validate",
    params(
        ("slug" = String, Path, description = "Utility provider slug"),
        ("ref" = String, Query, description = "Customer reference / meter / decoder number")
    ),
    responses(
        (status = 200, description = "Validation result", body = UtilityValidateResponse),
        (status = 400, description = "Missing reference", body = super::error::ErrorEnvelope),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn validate_utility_reference(
    Path(slug): Path<String>,
    Query(query): Query<UtilityValidateQuery>,
    State(state): State<HttpState>,
) -> Result<Json<UtilityValidateResponse>, ApiError> {
    if query.reference.trim().is_empty() {
        return Err(ApiError::bad_request("ref is required"));
    }

    let utility_type = slug_to_utility_type(&slug);
    let country = db::find_product_by_country_slug(&state.pool, DEFAULT_COUNTRY_ISO2, &slug)
        .await
        .ok()
        .flatten()
        .map(|product| product.country_iso2)
        .unwrap_or_else(|| String::from(DEFAULT_COUNTRY_ISO2));
    let reference = service_ref::normalize(&country, &utility_type, &query.reference)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;

    if utility_type == "school_fees" && !is_rrr(&reference) {
        return Err(ApiError::bad_request(
            "school fees need a 12 digit Remita RRR",
        ));
    }

    if let Some(dispatcher) = &state.provider_dispatcher {
        let request = crate::integrations::utility_provider::ValidateRefRequest {
            service_id: slug.clone(),
            billers_code: reference.clone(),
            meter_type: query.meter_type.clone(),
        };
        match dispatcher.validate_reference(&utility_type, &request).await {
            Ok(response) => {
                let amount_ngn = response
                    .settlement
                    .and_then(|settlement| settlement.to_ngn_whole().ok());
                let display = display_money(
                    &state,
                    None,
                    query.currency.as_deref(),
                    amount_ngn.and_then(|amount| Money::from_ngn_whole(amount).ok()),
                )
                .await;

                return Ok(Json(UtilityValidateResponse {
                    valid: response.is_valid,
                    customer_name: response.customer_name,
                    amount_ngn,
                    biller_name: response.biller_name,
                    product_name: response.product_name,
                    reference: response.reference,
                    display,
                }));
            }
            Err(error) if utility_type == "school_fees" => {
                return Err(remita_lookup_error(error));
            }
            Err(_) => {}
        }
    }

    let valid = !utility_type.is_empty();
    let display = display_money(&state, None, query.currency.as_deref(), None).await;

    Ok(Json(UtilityValidateResponse {
        valid,
        customer_name: if valid {
            Some(String::from("Validated Customer"))
        } else {
            None
        },
        amount_ngn: None,
        biller_name: None,
        product_name: None,
        reference: None,
        display,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/utilities/{slug}/variations",
    params(
        ("slug" = String, Path, description = "Utility provider slug")
    ),
    responses(
        (status = 200, description = "Variations list", body = [UtilityVariationItem]),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn list_utility_variations(
    Path(slug): Path<String>,
    State(state): State<HttpState>,
) -> Result<Json<Vec<UtilityVariationItem>>, ApiError> {
    let utility_type = slug_to_utility_type(&slug);

    if let Some(dispatcher) = &state.provider_dispatcher
        && let Ok(variations) = dispatcher.service_variations(&utility_type, &slug).await
    {
        return Ok(Json(
            variations
                .into_iter()
                .map(|v| UtilityVariationItem {
                    variation_code: v.variation_code,
                    name: v.name,
                    amount: v.amount,
                })
                .collect(),
        ));
    }

    Ok(Json(Vec::new()))
}

const MAX_REMITA_NOTIFICATIONS: usize = 10;

fn validate_create_order_payload(payload: &CreateOrderRequest) -> Result<(), ApiError> {
    if payload.utility_type.trim().is_empty() {
        return Err(ApiError::bad_request("utility_type is required"));
    }
    if !matches!(
        payload.utility_type.as_str(),
        "airtime"
            | "data"
            | "dstv"
            | "gotv"
            | "startimes"
            | "showmax"
            | "electricity"
            | "school_fees"
            | "waec"
            | "jamb"
            | "television"
            | "internet"
            | "landline"
            | "voip"
    ) {
        return Err(ApiError::bad_request("unsupported utility_type"));
    }
    if payload.utility_slug.trim().is_empty() {
        return Err(ApiError::bad_request("utility_slug is required"));
    }
    if payload.service_ref.trim().is_empty() {
        return Err(ApiError::bad_request("service_ref is required"));
    }
    if payload.amount_ngn.is_some() && payload.settlement.is_some() {
        return Err(ApiError::bad_request(
            "send either amount_ngn or settlement, not both",
        ));
    }
    if let Some(amount_ngn) = payload.amount_ngn
        && amount_ngn <= 0
    {
        return Err(ApiError::bad_request("amount_ngn must be greater than 0"));
    }
    if payload.zec_address_type != "shielded" {
        return Err(ApiError::bad_request(
            "transparent addresses are not available yet, choose shielded",
        ));
    }
    if let Some(meter_type) = payload.meter_type.as_deref()
        && !matches!(meter_type, "prepaid" | "postpaid")
    {
        return Err(ApiError::bad_request(
            "meter_type must be prepaid or postpaid",
        ));
    }
    if let Some(country) = payload.country_iso2.as_deref()
        && (country.len() != 2 || !country.chars().all(|c| c.is_ascii_alphabetic()))
    {
        return Err(ApiError::bad_request(
            "country_iso2 must be a two letter code",
        ));
    }
    Ok(())
}

pub(crate) fn generate_token(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

async fn build_readiness_report(state: &HttpState) -> ReadinessReport {
    let db_probe = match probe_db_connectivity(&state.pool).await {
        Ok(()) => ProbeStatus {
            healthy: true,
            detail: String::from("database connectivity ok"),
        },
        Err(error) => ProbeStatus {
            healthy: false,
            detail: format!("database probe failed: {error}"),
        },
    };

    let zcash_probe = match &state.zcash_client {
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
                detail: format!("zcash client probe failed: {error}"),
            },
        },
        None => ProbeStatus {
            healthy: false,
            detail: String::from("zcash client unavailable"),
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

async fn probe_db_connectivity(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

fn order_matches_user(order: &OrderRow, user: &crate::http::mw::AuthenticatedUser) -> bool {
    order.user_id == Some(user.user_id)
}

fn map_status_to_event(order: &OrderRow) -> Option<WsOrderEvent> {
    match order.status().ok()? {
        OrderStatus::AwaitingPayment => None,
        OrderStatus::PaymentDetected => Some(WsOrderEvent::PaymentDetected {
            confirmations: u16::try_from(order.confirmations).unwrap_or(u16::MAX),
            required: u16::try_from(order.required_confs).unwrap_or(u16::MAX),
        }),
        OrderStatus::PaymentConfirmed => Some(WsOrderEvent::PaymentConfirmed {
            confirmations: u16::try_from(order.confirmations).unwrap_or(u16::MAX),
        }),
        OrderStatus::AwaitingClaim => Some(WsOrderEvent::AwaitingClaim),
        OrderStatus::UtilityDispatching => Some(WsOrderEvent::Dispatching),
        OrderStatus::Completed => Some(WsOrderEvent::Completed {
            delivery_token: order.delivery_token.clone(),
            reference: order.id.to_string(),
        }),
        OrderStatus::Expired => Some(WsOrderEvent::Expired),
        OrderStatus::Failed => Some(WsOrderEvent::Failed {
            reason: String::from("order_failed"),
        }),
        OrderStatus::FlaggedForReview => Some(WsOrderEvent::Flagged {
            reason: String::from("flagged_for_review"),
        }),
        OrderStatus::Cancelled => Some(WsOrderEvent::Cancelled),
        OrderStatus::Reversed => Some(WsOrderEvent::Reversed),
    }
}

fn internal_err(error: impl std::fmt::Display) -> ApiError {
    tracing::error!(error = %error, "internal error");
    ApiError::internal("something went wrong on our side. Try again.")
}

fn remita_lookup_error(error: ProviderError) -> ApiError {
    match error.kind {
        ProviderErrorKind::Permanent => {
            tracing::warn!(detail = %error.message, "remita rejected the rrr lookup");
            ApiError::bad_request("we could not pay that RRR. Check the RRR and try again")
        }
        ProviderErrorKind::Transient | ProviderErrorKind::Outage => {
            tracing::warn!(detail = %error.message, "remita lookup unavailable");
            ApiError::service_unavailable("Remita is unavailable. Try again shortly.")
        }
        ProviderErrorKind::Unauthorized | ProviderErrorKind::InvalidResponse => {
            tracing::error!(detail = %error.message, "remita lookup failed");
            ApiError::service_unavailable("School fees are temporarily unavailable.")
        }
    }
}

fn is_rrr(reference: &str) -> bool {
    reference.len() == 12 && reference.chars().all(|c| c.is_ascii_digit())
}

fn is_school_fee_rrr_conflict(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<sqlx::Error>()
        .and_then(|error| match error {
            sqlx::Error::Database(database) => database
                .constraint()
                .map(|name| name == "ux_orders_active_school_fee_rrr"),
            _ => None,
        })
        .unwrap_or(false)
}

pub async fn webhook_inlomax(
    State(state): State<HttpState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(dispatcher) = &state.provider_dispatcher else {
        return Ok(StatusCode::SERVICE_UNAVAILABLE);
    };

    if !dispatcher.verify_webhook("airtime", &headers, &body) {
        return Err(ApiError::forbidden("invalid webhook signature"));
    }

    let event = match dispatcher.provider_for("airtime") {
        Ok(provider) => provider.parse_webhook_event(&body).map_err(internal_err)?,
        Err(_) => return Ok(StatusCode::SERVICE_UNAVAILABLE),
    };

    let order_id = if let Ok(id) = Uuid::parse_str(&event.provider_request_id) {
        Some(id)
    } else {
        db::find_order_by_provider_reference(&state.pool, &event.provider_request_id)
            .await
            .ok()
            .flatten()
            .map(|o| o.id)
    };

    let Some(order_id) = order_id else {
        tracing::warn!(reference = %event.provider_request_id, "inlomax webhook reference does not match any order");
        return Ok(StatusCode::OK);
    };

    match event.status {
        ProviderTxnStatus::Delivered => {
            match db::complete_order(
                &state.pool,
                order_id,
                event.token.as_deref(),
                Some(event.provider_request_id.as_str()),
            )
            .await
            {
                Ok(true) => {
                    let _ = state
                        .ws_hub
                        .broadcast_event(
                            order_id,
                            &WsOrderEvent::Completed {
                                delivery_token: event.token,
                                reference: order_id.to_string(),
                            },
                        )
                        .await;
                }
                Ok(false) => {
                    tracing::warn!(order_id = %order_id, "completion matched no dispatching order");
                }
                Err(error) => {
                    tracing::warn!(order_id = %order_id, error = %error, "completion failed");
                }
            }
        }
        ProviderTxnStatus::Failed => match db::fail_order(&state.pool, order_id).await {
            Ok(true) => {
                let _ = state
                    .ws_hub
                    .broadcast_event(
                        order_id,
                        &WsOrderEvent::Failed {
                            reason: String::from("provider_failed_webhook"),
                        },
                    )
                    .await;
            }
            Ok(false) => {
                tracing::warn!(order_id = %order_id, "failure matched no dispatching order");
            }
            Err(error) => {
                tracing::warn!(order_id = %order_id, error = %error, "failure failed");
            }
        },
        ProviderTxnStatus::Pending => {}
        ProviderTxnStatus::Reversed => {
            tracing::warn!(
                order_id = %order_id,
                "provider reported a reversal, only the dtone callback applies reversals"
            );
        }
    }

    Ok(StatusCode::OK)
}

pub async fn webhook_vtpass(
    State(state): State<HttpState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(dispatcher) = &state.provider_dispatcher else {
        return Ok(StatusCode::SERVICE_UNAVAILABLE);
    };

    if !dispatcher.verify_webhook("vtpass", &headers, &body) {
        return Err(ApiError::forbidden("invalid webhook signature"));
    }

    let event = dispatcher
        .provider_for("airtime")
        .map_err(|_| ApiError::internal("provider unavailable"))?
        .parse_webhook_event(&body)
        .map_err(internal_err)?;

    let order_id = match Uuid::parse_str(&event.provider_request_id) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(request_id = %event.provider_request_id, "webhook request_id is not a valid order UUID");
            return Ok(StatusCode::OK);
        }
    };

    match event.status {
        ProviderTxnStatus::Delivered => {
            match db::complete_order(
                &state.pool,
                order_id,
                event.token.as_deref(),
                Some(event.provider_request_id.as_str()),
            )
            .await
            {
                Ok(true) => {
                    let _ = state
                        .ws_hub
                        .broadcast_event(
                            order_id,
                            &WsOrderEvent::Completed {
                                delivery_token: event.token,
                                reference: order_id.to_string(),
                            },
                        )
                        .await;
                }
                Ok(false) => {
                    tracing::warn!(order_id = %order_id, "completion matched no dispatching order");
                }
                Err(error) => {
                    tracing::warn!(order_id = %order_id, error = %error, "completion failed");
                }
            }
        }
        ProviderTxnStatus::Failed => match db::fail_order(&state.pool, order_id).await {
            Ok(true) => {
                let _ = state
                    .ws_hub
                    .broadcast_event(
                        order_id,
                        &WsOrderEvent::Failed {
                            reason: String::from("provider_failed_webhook"),
                        },
                    )
                    .await;
            }
            Ok(false) => {
                tracing::warn!(order_id = %order_id, "failure matched no dispatching order");
            }
            Err(error) => {
                tracing::warn!(order_id = %order_id, error = %error, "failure failed");
            }
        },
        ProviderTxnStatus::Pending => {}
        ProviderTxnStatus::Reversed => {
            tracing::warn!(
                order_id = %order_id,
                "provider reported a reversal, only the dtone callback applies reversals"
            );
        }
    }

    Ok(StatusCode::OK)
}

pub async fn webhook_dtone(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    let Some(dispatcher) = state.provider_dispatcher.as_ref() else {
        return Ok(StatusCode::SERVICE_UNAVAILABLE);
    };
    let Some(dtone) = dispatcher.dtone() else {
        return Ok(StatusCode::SERVICE_UNAVAILABLE);
    };

    if !dtone.verify_webhook(&headers, &body) {
        return Err(ApiError::unauthorized("invalid dtone callback credentials"));
    }

    if !callback_address_allowed(dtone.as_ref(), peer, &headers) {
        tracing::warn!("dtone callback rejected from an address outside DTONE_CALLBACK_IPS");
        return Err(ApiError::unauthorized(
            "callback address is not allowlisted",
        ));
    }

    let hint = dtone
        .parse_callback(&body)
        .map_err(|_| ApiError::bad_request("invalid dtone callback body"))?;

    let transaction = match dtone.requery_transaction(&hint.external_id).await {
        Ok(Some(transaction)) => transaction,
        Ok(None) => {
            tracing::warn!(
                external_id = %hint.external_id,
                "dtone callback requery found no transaction"
            );
            return Ok(StatusCode::OK);
        }
        Err(error) => {
            state
                .observability
                .metrics()
                .record_provider_error(crate::integrations::dtone::PROVIDER_NAME);
            tracing::warn!(
                external_id = %hint.external_id,
                error = %error,
                "dtone callback requery failed, leaving the order untouched"
            );
            return Err(ApiError::service_unavailable("dtone requery failed"));
        }
    };

    let Some(class_id) = transaction.class_id else {
        tracing::warn!(
            external_id = %hint.external_id,
            "dtone callback transaction has no status class"
        );
        return Ok(StatusCode::OK);
    };

    if transaction.provider_transaction_id.is_empty() {
        tracing::warn!(
            external_id = %hint.external_id,
            "dtone callback transaction has no provider id"
        );
        return Ok(StatusCode::OK);
    }

    let order = match Uuid::parse_str(&hint.external_id) {
        Ok(order_id) => db::get_order_by_id(&state.pool, order_id)
            .await
            .map_err(internal_err)?,
        Err(_) => db::find_order_for_webhook(&state.pool, &hint.external_id)
            .await
            .map_err(internal_err)?,
    };

    let Some(order) = order else {
        tracing::warn!(external_id = %hint.external_id, "dtone callback matches no order");
        return Ok(StatusCode::OK);
    };

    let detail = serde_json::json!({
        "provider": crate::integrations::dtone::PROVIDER_NAME,
        "provider_transaction_id": transaction.provider_transaction_id,
        "provider_status_class": class_id,
    });

    let transition = match transaction.status {
        ProviderTxnStatus::Delivered => webhook_transition(
            &order,
            OrderStatus::Completed,
            "provider_completed",
            transaction.token.clone(),
            detail,
        ),
        ProviderTxnStatus::Failed => webhook_transition(
            &order,
            OrderStatus::Failed,
            "provider_failed_webhook",
            None,
            detail,
        ),
        ProviderTxnStatus::Reversed | ProviderTxnStatus::Pending => None,
    };

    let event_key = format!("{}:{}", transaction.provider_transaction_id, class_id);
    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;
    let outcome = db::record_webhook_and_transition(
        &mut tx,
        crate::integrations::dtone::PROVIDER_NAME,
        &event_key,
        order.id,
        transition.as_ref(),
    )
    .await
    .map_err(internal_err)?;

    let mut reversed = false;
    if transaction.status == ProviderTxnStatus::Reversed
        && outcome != db::WebhookTransitionOutcome::Replayed
    {
        reversed = db::mark_order_reversed_tx(&mut tx, order.id, &class_id.to_string(), None)
            .await
            .map_err(internal_err)?;
    }

    tx.commit().await.map_err(internal_err)?;

    tracing::info!(
        order_id = %order.id,
        external_id = %hint.external_id,
        class_id,
        status = ?transaction.status,
        outcome = ?outcome,
        "dtone callback processed"
    );

    if transaction.status == ProviderTxnStatus::Failed
        && outcome == db::WebhookTransitionOutcome::Applied
    {
        let message = transaction
            .raw
            .pointer("/status/message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("the provider declined the transaction");
        db::record_failure(&state.pool, order.id, &class_id.to_string(), message)
            .await
            .map_err(internal_err)?;
    }

    if reversed && let Some(user_id) = order.user_id {
        let _ = db::create_notification(
            &state.pool,
            user_id,
            Some(order.id),
            "order_reversed",
            "Order Reversed",
            &format!(
                "Your {} payment was reversed by the provider. Support will refund it to {}.",
                order.utility_slug, order.deposit_address
            ),
            serde_json::json!({
                "utility_slug": order.utility_slug,
                "refund_address": order.deposit_address,
                "provider_transaction_id": transaction.provider_transaction_id,
            }),
        )
        .await;
    }

    if outcome == db::WebhookTransitionOutcome::Applied || reversed {
        let event = match transaction.status {
            ProviderTxnStatus::Delivered => Some(WsOrderEvent::Completed {
                delivery_token: transaction.token.clone(),
                reference: order.id.to_string(),
            }),
            ProviderTxnStatus::Failed => Some(WsOrderEvent::Failed {
                reason: String::from("provider_failed_webhook"),
            }),
            ProviderTxnStatus::Reversed => Some(WsOrderEvent::Reversed),
            ProviderTxnStatus::Pending => None,
        };

        if let Some(event) = event {
            let _ = state.ws_hub.broadcast_event(order.id, &event).await;
        }
    }

    Ok(StatusCode::OK)
}

fn callback_address_allowed(
    dtone: &crate::integrations::dtone::DtoneClient,
    peer: SocketAddr,
    headers: &axum::http::HeaderMap,
) -> bool {
    if dtone.is_allowed_callback_ip(peer.ip()) {
        return true;
    }

    if !is_trusted_proxy(peer.ip()) {
        return false;
    }

    headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<IpAddr>().ok())
        .map(|address| dtone.is_allowed_callback_ip(address))
        .unwrap_or(false)
}

fn is_trusted_proxy(address: IpAddr) -> bool {
    address.is_loopback()
}

fn webhook_transition(
    order: &OrderRow,
    to: OrderStatus,
    event: &str,
    delivery_token: Option<String>,
    detail: serde_json::Value,
) -> Option<db::WebhookTransition> {
    let from = OrderStatus::from_db(&order.status)?;
    let transition = OrderStatusTransition::new(from, to).ok()?;
    let provider_reference = detail
        .get("provider_transaction_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);

    Some(db::WebhookTransition {
        from_status: transition.from.as_db().to_owned(),
        to_status: transition.to.as_db().to_owned(),
        event: event.to_owned(),
        delivery_token,
        provider_reference,
        detail,
    })
}

pub async fn webhook_remita(
    State(state): State<HttpState>,
    body: axum::body::Bytes,
) -> Result<StatusCode, ApiError> {
    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| ApiError::bad_request(format!("invalid remita webhook body: {error}")))?;

    for notification in remita_notifications(payload) {
        let Some(rrr) = notification.get("rrr").and_then(serde_json::Value::as_str) else {
            continue;
        };

        let Some(order) = db::find_order_by_provider_reference(&state.pool, rrr)
            .await
            .map_err(internal_err)?
        else {
            continue;
        };

        if order.utility_type != "school_fees" {
            tracing::warn!(order_id = %order.id, "remita webhook matched a non school fees order");
            continue;
        }

        if let Some(claimed_amount) = notification
            .get("amount")
            .and_then(serde_json::Value::as_f64)
            .map(|amount| amount.round() as i64)
            && claimed_amount != order.amount_ngn
        {
            tracing::warn!(
                order_id = %order.id,
                claimed_amount,
                expected_amount = order.amount_ngn,
                "remita webhook amount does not match the order"
            );
            continue;
        }

        let Some(dispatcher) = state.provider_dispatcher.as_ref() else {
            continue;
        };

        let result = match dispatcher.requery("school_fees", rrr).await {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(order_id = %order.id, error = %error, "remita webhook lookup failed");
                continue;
            }
        };

        if result.status != ProviderTxnStatus::Delivered {
            continue;
        }

        let token = result.token.as_deref();

        let accepted = order
            .metadata
            .get("provider_accepted")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !accepted {
            let flagged = db::update_order_status_cas(
                &state.pool,
                order.id,
                "utility_dispatching",
                "flagged_for_review",
                "provider_paid_without_acceptance",
                serde_json::json!({ "rrr": rrr, "source": "webhook" }),
            )
            .await
            .map_err(internal_err)?;

            if flagged {
                tracing::error!(order_id = %order.id, "RRR paid without an accepted process call, flagged");
                let _ = state
                    .ws_hub
                    .broadcast_event(
                        order.id,
                        &WsOrderEvent::Flagged {
                            reason: String::from("provider_paid_without_acceptance"),
                        },
                    )
                    .await;
            }

            continue;
        }

        let completed = if order.status == "flagged_for_review" {
            db::complete_flagged_school_fee_order(&state.pool, order.id, token, Some(rrr))
                .await
                .map_err(internal_err)?
        } else {
            db::complete_order(&state.pool, order.id, token, Some(rrr))
                .await
                .map_err(internal_err)?
        };

        if !completed {
            continue;
        }

        let _ = state
            .ws_hub
            .broadcast_event(
                order.id,
                &WsOrderEvent::Completed {
                    delivery_token: token.map(str::to_owned),
                    reference: order.id.to_string(),
                },
            )
            .await;

        if let Some(user_id) = order.user_id {
            let _ = db::create_notification(
                &state.pool,
                user_id,
                Some(order.id),
                "order_completed",
                "Order Completed",
                "Your school fees payment was delivered.",
                serde_json::json!({ "utility_slug": order.utility_slug }),
            )
            .await;
        }
    }

    Ok(StatusCode::OK)
}

fn remita_notifications(payload: serde_json::Value) -> Vec<serde_json::Value> {
    let items = match payload {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![payload],
        _ => Vec::new(),
    };

    let mut seen = std::collections::HashSet::new();
    let mut notifications = Vec::new();

    for item in items {
        let Some(rrr) = item.get("rrr").and_then(serde_json::Value::as_str) else {
            continue;
        };

        if !is_rrr(rrr) || !seen.insert(rrr.to_owned()) {
            continue;
        }

        notifications.push(item);

        if notifications.len() == MAX_REMITA_NOTIFICATIONS {
            break;
        }
    }

    notifications
}

pub(crate) fn slug_to_utility_type(slug: &str) -> String {
    match slug {
        "mtn" | "airtel" | "glo" | "9mobile" => String::from("airtime"),
        "mtn-data" | "airtel-data" | "glo-data" | "9mobile-data" => String::from("data"),
        "dstv" => String::from("dstv"),
        "gotv" => String::from("gotv"),
        "startimes" => String::from("startimes"),
        "showmax" => String::from("showmax"),
        s if s.ends_with("-electric") => String::from("electricity"),
        "waec-registration" | "waec-result-checker" => String::from("waec"),
        "jamb" => String::from("jamb"),
        "school-fees" => String::from("school_fees"),
        _ => String::new(),
    }
}

pub(crate) async fn get_or_create_rate_snapshot(
    pool: &PgPool,
    rate: &CurrentRate,
) -> Result<Uuid, anyhow::Error> {
    let input = db::PersistRateSnapshotInput {
        zec_ngn: rate.zec_ngn,
        zec_usd: rate.zec_usd,
        usd_ngn: rate.usd_ngn,
        usd_kes: rate.usd_kes,
        usd_ghs: rate.usd_ghs,
        usd_zar: rate.usd_zar,
        usd_egp: rate.usd_egp,
        coingecko_zec_ngn: None,
        binance_zec_usd: None,
        kraken_zec_usd: None,
        coinbase_zec_usd: None,
        sources_used: vec![String::from("cache")],
        fetched_at: rate.updated_at,
    };
    db::persist_rate_snapshot(pool, &input).await
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DisplayContext {
    currency: CurrencyCode,
    rate: Option<(Decimal, Decimal)>,
}

impl DisplayContext {
    pub async fn load(
        state: &HttpState,
        auth_user: Option<&crate::http::mw::AuthenticatedUser>,
        override_currency: Option<&str>,
    ) -> Self {
        let currency = DisplayMoney::target_currency(
            override_currency,
            auth_user
                .map(|user| user.preferred_currency)
                .unwrap_or(CurrencyCode::Ngn),
        );

        if currency == CurrencyCode::Ngn {
            return Self {
                currency,
                rate: None,
            };
        }

        let window = Duration::minutes(state.rate_lock_minutes);
        let now = Utc::now();

        let bridge = match db::latest_usd_ngn(&state.pool).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(error = %error, "display bridge rate read failed");
                None
            }
        }
        .filter(|(usd_ngn, fetched_at)| *usd_ngn > Decimal::ZERO && *fetched_at + window > now);

        let Some((usd_ngn, _)) = bridge else {
            return Self {
                currency,
                rate: None,
            };
        };

        let per_usd = if currency == CurrencyCode::Usd {
            Some(Decimal::ONE)
        } else {
            match db::find_fx_rate(&state.pool, currency.iso()).await {
                Ok(value) => value
                    .filter(|row| row.per_usd > Decimal::ZERO && row.fetched_at + window > now)
                    .map(|row| row.per_usd),
                Err(error) => {
                    tracing::warn!(error = %error, "display fx rate read failed");
                    None
                }
            }
        };

        Self {
            currency,
            rate: per_usd.map(|per_usd| (usd_ngn, per_usd)),
        }
    }

    pub fn project(&self, amount_ngn: Option<Money>) -> DisplayMoney {
        DisplayMoney::new(self.currency, amount_ngn, self.rate)
    }
}

pub(crate) async fn display_money(
    state: &HttpState,
    auth_user: Option<&crate::http::mw::AuthenticatedUser>,
    override_currency: Option<&str>,
    amount_ngn: Option<Money>,
) -> DisplayMoney {
    DisplayContext::load(state, auth_user, override_currency)
        .await
        .project(amount_ngn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(amount_ngn: i64) -> CreateOrderRequest {
        CreateOrderRequest {
            utility_type: String::from("airtime"),
            utility_slug: String::from("mtn"),
            service_ref: String::from("08000000000"),
            amount_ngn: Some(amount_ngn),
            zec_address_type: String::from("shielded"),
            variation_code: None,
            meter_type: None,
            country_iso2: None,
            product_id: None,
            settlement: None,
            inputs: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn rejects_an_amount_and_a_settlement_in_the_same_request() {
        let mut request = payload(5000);
        request.settlement = Some(super::super::types::SettlementRequest {
            currency: String::from("XOF"),
            amount: String::from("1000"),
        });
        assert!(matches!(
            validate_create_order_payload(&request),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn parses_remita_webhook_array_and_object() {
        let array = serde_json::json!([
            {"rrr": "171523991694"},
            {"rrr": "141523991664"}
        ]);
        assert_eq!(remita_notifications(array).len(), 2);

        let object = serde_json::json!({"rrr": "171523991694"});
        assert_eq!(remita_notifications(object).len(), 1);

        assert!(remita_notifications(serde_json::json!("text")).is_empty());
    }

    #[test]
    fn drops_malformed_and_repeated_remita_notifications() {
        let notifications = serde_json::json!([
            {"rrr": "171523991694"},
            {"rrr": "171523991694"},
            {"rrr": "17152399169"},
            {"rrr": "../../admin"},
            {"amount": 1000},
            {"rrr": "141523991664"}
        ]);

        let parsed = remita_notifications(notifications);
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn caps_remita_notification_batch() {
        let items: Vec<serde_json::Value> = (0..50)
            .map(|index| serde_json::json!({ "rrr": format!("{:012}", index) }))
            .collect();

        assert_eq!(
            remita_notifications(serde_json::Value::Array(items)).len(),
            MAX_REMITA_NOTIFICATIONS
        );
    }

    #[test]
    fn rejects_non_positive_amounts() {
        assert!(matches!(
            validate_create_order_payload(&payload(0)),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_create_order_payload(&payload(-1)),
            Err(ApiError::BadRequest(_))
        ));
    }

    fn order_with_status(status: OrderStatus) -> OrderRow {
        OrderRow {
            id: Uuid::new_v4(),
            status: status.as_db().to_owned(),
            access_token_hash: String::new(),
            utility_type: String::from("airtime"),
            utility_slug: String::from("mtn"),
            service_ref: String::new(),
            amount_ngn: 0,
            country_iso2: String::from("NG"),
            product_id: None,
            provider_external_id: None,
            provider_transaction_id: None,
            provider_status: None,
            failure_code: None,
            failure_message: None,
            settlement: None,
            cost: None,
            inputs: std::collections::BTreeMap::new(),
            fulfilment_kind: None,
            flag_reason: None,
            deposit_address: String::new(),
            address_type: String::from("shielded"),
            zec_amount: Decimal::ZERO,
            zec_rate_id: Uuid::new_v4(),
            txid: None,
            confirmations: 0,
            required_confs: 2,
            total_received: None,
            created_at: Utc::now(),
            expires_at: Utc::now(),
            confirmed_at: None,
            completed_at: None,
            vtpass_request_id: None,
            delivery_token: None,
            ip_hash: None,
            metadata: serde_json::json!({}),
            variation_code: None,
            provider: None,
            customer_name: None,
            user_id: None,
        }
    }

    #[test]
    fn maps_a_reversed_order_to_a_reversed_event() {
        assert_eq!(
            map_status_to_event(&order_with_status(OrderStatus::Reversed)),
            Some(WsOrderEvent::Reversed)
        );
    }

    #[test]
    fn velocity_key_is_bounded() {
        let long_type = "a".repeat(100);
        let long_slug = "b".repeat(100);
        let long_ref = "c".repeat(1000);
        let key = velocity_key(&long_type, &long_slug, &long_ref);
        let parts: Vec<&str> = key.split(':').collect();

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 32);
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 64);
    }

    #[test]
    fn maps_flagged_and_cancelled_statuses_to_initial_events() {
        assert_eq!(
            map_status_to_event(&order_with_status(OrderStatus::FlaggedForReview)),
            Some(WsOrderEvent::Flagged {
                reason: String::from("flagged_for_review"),
            })
        );
        assert_eq!(
            map_status_to_event(&order_with_status(OrderStatus::Cancelled)),
            Some(WsOrderEvent::Cancelled)
        );
        assert_eq!(
            map_status_to_event(&order_with_status(OrderStatus::AwaitingPayment)),
            None
        );
    }
}
