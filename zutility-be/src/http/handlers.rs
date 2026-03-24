use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    response::Response,
};
use chrono::{Duration, Utc};
use rand::{RngExt, distr::Alphanumeric};
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    domain::order::OrderStatus,
    http::auth,
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
        }
    }
}

pub async fn create_order(
    State(state): State<HttpState>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<CreateOrderResponse>, ApiError> {
    validate_create_order_payload(&payload)?;

    let order_id = Uuid::new_v4();
    let order_access_token = generate_token(48);
    let access_token_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &order_access_token)
            .map_err(ApiError::internal)?;

    let required_confirmations = if payload.zec_address_type == "shielded" {
        10
    } else {
        3
    };
    let zec_amount = Decimal::new(payload.amount_ngn, 0) / Decimal::new(150_000, 0);
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
        utility_type: order.utility_type,
        utility_slug: order.utility_slug,
        service_ref: order.service_ref,
        amount_ngn: order.amount_ngn,
        zec_amount: order.zec_amount.round_dp(8).to_string(),
        expires_at: order.expires_at,
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
    let updated_at = Utc::now();
    let valid_until = updated_at + Duration::minutes(state.rate_lock_minutes);

    Ok(Json(RateResponse {
        zec_ngn: String::from("150000.0000"),
        zec_usd: String::from("100.0000"),
        updated_at,
        valid_until,
    }))
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
            token: None,
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
