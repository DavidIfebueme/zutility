use std::collections::HashMap;

use axum::{
    Json,
    extract::{Extension, Path, State},
};
use chrono::{Duration, Utc};
use rand::RngExt;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    db::{self, CreateOrderInput, NewGift, OrderStatusTransitionRecord},
    domain::{
        gift::{
            GiftStatus, MAX_OUTSTANDING_GIFT_NGN, is_giftable_utility, validate_claim_window_hours,
            validate_gift_amount_ngn,
        },
        order::{OrderStatus, OrderStatusTransition},
    },
    http::{
        auth,
        error::ApiError,
        handlers::{HttpState, generate_token, get_or_create_rate_snapshot},
        mw::AuthenticatedUser,
        types::{
            CancelOrderResponse, ClaimGiftRequest, CreateGiftRequest, CreateGiftResponse,
            CreateOrderResponse, GiftSummary, GiftView, RotateGiftCodeResponse,
        },
    },
    integrations::utility_provider::ValidateRefRequest,
};

const MAX_GIFT_MESSAGE_CHARS: usize = 240;
const GIFT_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
const GIFT_CODE_LENGTH: usize = 10;
const FRIENDLY_SENDER_NAME: &str = "A friend";

fn internal_err(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

fn generate_gift_code() -> String {
    let mut rng = rand::rng();
    (0..GIFT_CODE_LENGTH)
        .map(|_| GIFT_CODE_ALPHABET[rng.random_range(0..GIFT_CODE_ALPHABET.len())] as char)
        .collect()
}

fn resolve_sender_name(display_name: Option<String>, hide_sender_name: bool) -> Option<String> {
    if hide_sender_name {
        return None;
    }

    display_name.filter(|value| !value.trim().is_empty())
}

async fn validate_service_ref(
    state: &HttpState,
    utility_type: &str,
    utility_slug: &str,
    service_ref: &str,
    label: &str,
    strict: bool,
) -> Result<Option<String>, ApiError> {
    let Some(dispatcher) = state.provider_dispatcher.as_ref() else {
        return Ok(None);
    };

    let request = ValidateRefRequest {
        service_id: utility_slug.to_owned(),
        billers_code: service_ref.to_owned(),
    };

    match dispatcher.validate_reference(utility_type, &request).await {
        Ok(response) if !response.is_valid => Err(ApiError::bad_request(format!(
            "the {label} account did not validate"
        ))),
        Ok(response) => Ok(response.customer_name),
        Err(error) => {
            tracing::warn!(
                utility_type = %utility_type,
                utility_slug = %utility_slug,
                label = %label,
                error = %error,
                "provider reference validation unavailable"
            );

            if strict {
                return Err(ApiError::bad_request(
                    "we could not verify that account right now. Try again in a moment.",
                ));
            }

            Ok(None)
        }
    }
}

fn build_gift_view(gift: &db::GiftRow, order: Option<&db::OrderRow>) -> Result<GiftView, ApiError> {
    let status = gift
        .status()
        .ok_or_else(|| ApiError::internal("unknown gift status"))?;

    let sender_name = if gift.hide_sender_name {
        String::from(FRIENDLY_SENDER_NAME)
    } else {
        gift.sender_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| String::from(FRIENDLY_SENDER_NAME))
    };

    let zec_amount = order
        .as_ref()
        .map(|row| row.zec_amount.round_dp(8).to_string());

    let (order_status, confirmations, required_confirmations, delivery_token) = match order {
        Some(order) => (
            order.status().ok(),
            u16::try_from(order.confirmations).unwrap_or(0),
            u16::try_from(order.required_confs).unwrap_or(0),
            order.delivery_token.clone(),
        ),
        None => (None, 0, 0, None),
    };

    Ok(GiftView {
        gift_id: gift.id,
        zec_amount,
        utility_type: gift.utility_type.clone(),
        utility_slug: gift.utility_slug.clone(),
        amount_ngn: gift.amount_ngn,
        message: gift.message.clone(),
        sender_name,
        status,
        claimed: status == GiftStatus::Claimed,
        claim_expires_at: gift.claim_expires_at,
        order_status,
        confirmations,
        required_confirmations,
        delivery_token,
    })
}

async fn load_gift_by_code(state: &HttpState, code: &str) -> Result<db::GiftRow, ApiError> {
    let code_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, code).map_err(internal_err)?;

    db::find_gift_by_code_hash(&state.pool, &code_hash)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("gift not found"))
}

#[utoipa::path(
    post,
    path = "/api/v1/gifts",
    request_body = CreateGiftRequest,
    responses(
        (status = 200, description = "Gift created", body = CreateGiftResponse),
        (status = 400, description = "Validation error", body = super::error::ErrorEnvelope),
        (status = 409, description = "Outstanding gift limit reached", body = super::error::ErrorEnvelope)
    )
)]
pub async fn create_gift(
    State(state): State<HttpState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateGiftRequest>,
) -> Result<Json<CreateGiftResponse>, ApiError> {
    if payload.utility_type.trim().is_empty() {
        return Err(ApiError::bad_request("utility_type is required"));
    }
    if payload.utility_slug.trim().is_empty() {
        return Err(ApiError::bad_request("utility_slug is required"));
    }
    if !is_giftable_utility(payload.utility_type.trim()) {
        return Err(ApiError::bad_request(
            "gifting is not available for this utility",
        ));
    }
    if !matches!(
        payload.zec_address_type.as_str(),
        "shielded" | "transparent"
    ) {
        return Err(ApiError::bad_request(
            "zec_address_type must be shielded or transparent",
        ));
    }
    validate_gift_amount_ngn(payload.amount_ngn)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    validate_claim_window_hours(payload.claim_window_hours)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;

    let message = payload
        .message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(ref value) = message
        && value.chars().count() > MAX_GIFT_MESSAGE_CHARS
    {
        return Err(ApiError::bad_request(format!(
            "message cannot exceed {MAX_GIFT_MESSAGE_CHARS} characters"
        )));
    }

    let fallback_service_ref = payload
        .fallback_service_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if payload.utility_type.trim() == "electricity" && fallback_service_ref.is_none() {
        return Err(ApiError::bad_request(
            "electricity gifts require a fallback meter number",
        ));
    }

    if let Some(ref fallback) = fallback_service_ref {
        let _ = validate_service_ref(
            &state,
            payload.utility_type.trim(),
            payload.utility_slug.trim(),
            fallback,
            "fallback",
            false,
        )
        .await?;
    }

    let rate = state.rate_cache.read().await.clone();
    if rate.zec_ngn <= Decimal::ZERO {
        return Err(ApiError::internal(
            "rate data unavailable — please try again in a moment",
        ));
    }

    let zec_amount = Decimal::new(payload.amount_ngn, 0) / rate.zec_ngn;
    let required_confirmations = if payload.zec_address_type == "shielded" {
        state.required_confs_shielded
    } else {
        state.required_confs_transparent
    };
    let expires_at = Utc::now() + Duration::minutes(state.order_expiry_minutes);
    let rate_snapshot_id = get_or_create_rate_snapshot(&state.pool, &rate)
        .await
        .map_err(internal_err)?;

    let user = db::find_user_by_id(&state.pool, auth_user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    let hide_sender_name = payload.hide_sender_name.unwrap_or(false);
    let sender_name = resolve_sender_name(user.display_name, hide_sender_name);
    let claim_window_hours = i32::try_from(payload.claim_window_hours)
        .map_err(|_| ApiError::bad_request("claim window is out of range"))?;

    let order_access_token = generate_token(48);
    let access_token_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &order_access_token)
            .map_err(internal_err)?;
    let gift_code = generate_gift_code();
    let gift_code_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &gift_code).map_err(internal_err)?;

    let input = CreateOrderInput {
        access_token_hash,
        utility_type: payload.utility_type.trim().to_owned(),
        utility_slug: payload.utility_slug.trim().to_owned(),
        service_ref: fallback_service_ref.clone().unwrap_or_default(),
        amount_ngn: payload.amount_ngn,
        address_type: payload.zec_address_type.clone(),
        zec_amount,
        zec_rate_id: rate_snapshot_id,
        required_confs: i32::from(required_confirmations),
        expires_at,
        ip_hash: None,
        metadata: serde_json::json!({
            "utility_slug": payload.utility_slug,
            "variation_code": payload.variation_code,
            "gift": true,
        }),
        variation_code: payload.variation_code.clone(),
    };

    let network = state.zcash_network.as_str();
    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;

    let outstanding = db::lock_user_and_sum_open_gifts(&mut tx, auth_user.user_id)
        .await
        .map_err(internal_err)?;
    if outstanding.saturating_add(payload.amount_ngn) > MAX_OUTSTANDING_GIFT_NGN {
        return Err(ApiError::conflict(
            "outstanding gift limit reached. Claim, cancel, or wait for a gift to expire first.",
        ));
    }

    let (order_id, deposit_address) =
        db::insert_order_with_claimed_address(&mut tx, &input, network)
            .await
            .map_err(internal_err)?;
    db::set_order_user_id_tx(&mut tx, order_id, auth_user.user_id)
        .await
        .map_err(internal_err)?;

    let gift_id = db::insert_gift_tx(
        &mut tx,
        &NewGift {
            code_hash: gift_code_hash,
            sender_user_id: auth_user.user_id,
            order_id,
            utility_type: payload.utility_type.trim().to_owned(),
            utility_slug: payload.utility_slug.trim().to_owned(),
            amount_ngn: payload.amount_ngn,
            claim_window_hours,
            message,
            sender_name,
            hide_sender_name,
            fallback_service_ref,
            claim_expires_at: expires_at,
        },
    )
    .await
    .map_err(internal_err)?;

    tx.commit().await.map_err(internal_err)?;

    Ok(Json(CreateGiftResponse {
        gift_id,
        gift_url: format!(
            "{}/gift/{gift_code}",
            state.app_base_url.trim_end_matches('/')
        ),
        gift_code,
        claim_window_hours: payload.claim_window_hours,
        order: CreateOrderResponse {
            order_id,
            order_access_token,
            deposit_address: deposit_address.clone(),
            zec_amount: zec_amount.round_dp(8).to_string(),
            expires_at,
            qr_data: format!("zcash:{deposit_address}?amount={}", zec_amount.round_dp(8)),
            required_confirmations,
            address_type: payload.zec_address_type.clone(),
            utility_slug: payload.utility_slug.trim().to_owned(),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/gifts",
    responses(
        (status = 200, description = "Gifts sent by the authenticated user", body = Vec<GiftSummary>)
    )
)]
pub async fn list_gifts(
    State(state): State<HttpState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<GiftSummary>>, ApiError> {
    let gifts = db::list_gifts_by_sender(&state.pool, auth_user.user_id)
        .await
        .map_err(internal_err)?;
    let statuses: HashMap<Uuid, String> =
        db::list_order_statuses_by_user(&state.pool, auth_user.user_id)
            .await
            .map_err(internal_err)?
            .into_iter()
            .collect();

    let mut summaries = Vec::with_capacity(gifts.len());
    for gift in gifts {
        let status = gift
            .status()
            .ok_or_else(|| ApiError::internal("unknown gift status"))?;
        let order_status = gift
            .order_id
            .and_then(|order_id| statuses.get(&order_id))
            .and_then(|value| OrderStatus::from_db(value));

        summaries.push(GiftSummary {
            gift_id: gift.id,
            order_id: gift.order_id,
            utility_type: gift.utility_type,
            utility_slug: gift.utility_slug,
            amount_ngn: gift.amount_ngn,
            status,
            order_status,
            claimed_service_ref: gift.claimed_service_ref,
            claim_window_hours: i64::from(gift.claim_window_hours),
            claim_expires_at: gift.claim_expires_at,
            created_at: gift.created_at,
        });
    }

    Ok(Json(summaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/gifts/{code}",
    params(("code" = String, Path, description = "Gift claim code")),
    responses(
        (status = 200, description = "Gift preview", body = GiftView),
        (status = 404, description = "Gift not found", body = super::error::ErrorEnvelope)
    )
)]
pub async fn get_gift(
    Path(code): Path<String>,
    State(state): State<HttpState>,
) -> Result<Json<GiftView>, ApiError> {
    let gift = load_gift_by_code(&state, &code).await?;
    let order = match gift.order_id {
        Some(order_id) => db::get_order_by_id(&state.pool, order_id)
            .await
            .map_err(internal_err)?,
        None => None,
    };

    Ok(Json(build_gift_view(&gift, order.as_ref())?))
}

#[utoipa::path(
    post,
    path = "/api/v1/gifts/{code}/claim",
    request_body = ClaimGiftRequest,
    params(("code" = String, Path, description = "Gift claim code")),
    responses(
        (status = 200, description = "Gift claimed", body = GiftView),
        (status = 400, description = "Invalid account reference", body = super::error::ErrorEnvelope),
        (status = 404, description = "Gift not found", body = super::error::ErrorEnvelope),
        (status = 409, description = "Gift already claimed, cancelled, or expired", body = super::error::ErrorEnvelope)
    )
)]
pub async fn claim_gift(
    Path(code): Path<String>,
    State(state): State<HttpState>,
    Json(payload): Json<ClaimGiftRequest>,
) -> Result<Json<GiftView>, ApiError> {
    let gift = load_gift_by_code(&state, &code).await?;

    match gift.status() {
        Some(GiftStatus::Open) => {}
        Some(GiftStatus::Claimed) => {
            return Err(ApiError::conflict("this gift has already been claimed"));
        }
        Some(GiftStatus::Expired) => {
            return Err(ApiError::conflict("this gift has expired"));
        }
        Some(GiftStatus::Cancelled) => {
            return Err(ApiError::conflict("this gift was cancelled"));
        }
        None => return Err(ApiError::internal("unknown gift status")),
    }

    if let Some(claim_expires_at) = gift.claim_expires_at
        && claim_expires_at <= Utc::now()
    {
        let _ = db::expire_gift_cas(&state.pool, gift.id).await;
        return Err(ApiError::conflict("this gift has expired"));
    }

    let service_ref = payload.service_ref.trim();
    if service_ref.is_empty() {
        return Err(ApiError::bad_request("service_ref is required"));
    }

    let order_id = gift
        .order_id
        .ok_or_else(|| ApiError::internal("gift has no order"))?;

    let customer_name = validate_service_ref(
        &state,
        &gift.utility_type,
        &gift.utility_slug,
        service_ref,
        "recipient",
        true,
    )
    .await?;

    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;

    if !db::claim_gift_tx(&mut tx, gift.id, service_ref, customer_name.as_deref())
        .await
        .map_err(internal_err)?
    {
        return Err(ApiError::conflict("this gift has already been claimed"));
    }

    let locked_status = db::lock_order_status_tx(&mut tx, order_id)
        .await
        .map_err(internal_err)?;

    match locked_status.as_deref() {
        Some("awaiting_payment")
        | Some("payment_detected")
        | Some("payment_confirmed")
        | Some("awaiting_claim") => {}
        _ => {
            return Err(ApiError::conflict("this gift can no longer be claimed"));
        }
    }

    db::update_order_service_ref_tx(&mut tx, order_id, service_ref, customer_name.as_deref())
        .await
        .map_err(internal_err)?;

    if locked_status.as_deref() == Some("awaiting_claim") {
        let transition =
            OrderStatusTransition::new(OrderStatus::AwaitingClaim, OrderStatus::PaymentConfirmed)
                .map_err(internal_err)?;

        db::apply_order_status_transition(
            &mut tx,
            &OrderStatusTransitionRecord {
                order_id,
                transition,
                event: String::from("gift_claimed"),
                detail: serde_json::json!({ "claimed_service_ref": service_ref }),
            },
        )
        .await
        .map_err(internal_err)?;
    }

    tx.commit().await.map_err(internal_err)?;

    let _ = db::create_notification(
        &state.pool,
        gift.sender_user_id,
        Some(order_id),
        "gift_claimed",
        "Gift claimed",
        &format!(
            "Your {} gift was claimed and is being delivered.",
            gift.utility_slug
        ),
        serde_json::json!({ "gift_id": gift.id, "utility_slug": gift.utility_slug }),
    )
    .await;

    let refreshed_gift = db::find_gift_by_id(&state.pool, gift.id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::internal("gift not found after claim"))?;
    let refreshed_order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?;

    Ok(Json(build_gift_view(
        &refreshed_gift,
        refreshed_order.as_ref(),
    )?))
}

#[utoipa::path(
    post,
    path = "/api/v1/gifts/{gift_id}/rotate-code",
    params(("gift_id" = Uuid, Path, description = "Gift id")),
    responses(
        (status = 200, description = "Fresh gift code", body = RotateGiftCodeResponse),
        (status = 403, description = "Not the sender", body = super::error::ErrorEnvelope),
        (status = 404, description = "Gift not found", body = super::error::ErrorEnvelope),
        (status = 409, description = "Gift is no longer open", body = super::error::ErrorEnvelope)
    )
)]
pub async fn rotate_gift_code(
    Path(gift_id): Path<Uuid>,
    State(state): State<HttpState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<RotateGiftCodeResponse>, ApiError> {
    let gift = db::find_gift_by_id(&state.pool, gift_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("gift not found"))?;

    if gift.sender_user_id != auth_user.user_id {
        return Err(ApiError::forbidden("access denied"));
    }

    if gift.status() != Some(GiftStatus::Open) {
        return Err(ApiError::conflict("only an open gift can get a new link"));
    }

    let gift_code = generate_gift_code();
    let code_hash =
        auth::hash_order_token(&state.order_token_hmac_secret, &gift_code).map_err(internal_err)?;

    if !db::set_gift_code_hash(&state.pool, gift.id, &code_hash)
        .await
        .map_err(internal_err)?
    {
        return Err(ApiError::conflict("this gift is no longer open"));
    }

    Ok(Json(RotateGiftCodeResponse {
        gift_url: format!(
            "{}/gift/{gift_code}",
            state.app_base_url.trim_end_matches('/')
        ),
        gift_code,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/gifts/{gift_id}/cancel",
    params(("gift_id" = Uuid, Path, description = "Gift id")),
    responses(
        (status = 200, description = "Gift cancelled", body = CancelOrderResponse),
        (status = 403, description = "Not the sender", body = super::error::ErrorEnvelope),
        (status = 404, description = "Gift not found", body = super::error::ErrorEnvelope),
        (status = 409, description = "Gift cannot be cancelled", body = super::error::ErrorEnvelope)
    )
)]
pub async fn cancel_gift(
    Path(gift_id): Path<Uuid>,
    State(state): State<HttpState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<CancelOrderResponse>, ApiError> {
    let gift = db::find_gift_by_id(&state.pool, gift_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::not_found("gift not found"))?;

    if gift.sender_user_id != auth_user.user_id {
        return Err(ApiError::forbidden("access denied"));
    }

    if gift.status() != Some(GiftStatus::Open) {
        return Err(ApiError::conflict("only an open gift can be cancelled"));
    }

    let order_id = gift
        .order_id
        .ok_or_else(|| ApiError::internal("gift has no order"))?;
    let order = db::get_order_by_id(&state.pool, order_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::internal("gift order not found"))?;

    if order.status().ok() != Some(OrderStatus::AwaitingPayment) {
        return Err(ApiError::conflict(
            "this gift has already been paid and can no longer be cancelled",
        ));
    }

    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;

    if !db::cancel_gift_tx(&mut tx, gift.id)
        .await
        .map_err(internal_err)?
    {
        return Err(ApiError::conflict("this gift is no longer open"));
    }

    let transition =
        OrderStatusTransition::new(OrderStatus::AwaitingPayment, OrderStatus::Cancelled)
            .map_err(internal_err)?;

    db::apply_order_status_transition(
        &mut tx,
        &OrderStatusTransitionRecord {
            order_id,
            transition,
            event: String::from("gift_cancelled"),
            detail: serde_json::json!({ "gift_id": gift.id }),
        },
    )
    .await
    .map_err(internal_err)?;

    tx.commit().await.map_err(internal_err)?;

    Ok(Json(CancelOrderResponse { success: true }))
}
