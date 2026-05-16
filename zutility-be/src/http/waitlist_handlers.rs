use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use rand::RngExt;
use sqlx::Row as _;

use crate::{
    db,
    http::auth,
    http::error::ApiError,
    http::handlers::HttpState,
    http::types::{
        WaitlistJoinRequest, WaitlistJoinResponse, WaitlistResendRequest, WaitlistStatsResponse,
        WaitlistVerifyRequest, WaitlistVerifyResponse,
    },
};

fn validate_email(email: &str) -> Result<(), ApiError> {
    if !auth::is_valid_email(email) {
        return Err(ApiError::bad_request("valid email is required"));
    }
    Ok(())
}

fn internal_err(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

fn generate_referral_code() -> String {
    let mut rng = rand::rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..8).map(|_| chars[rng.random_range(0..chars.len())]).collect()
}

fn extract_ip_hash(state: &HttpState, headers: &axum::http::HeaderMap) -> Option<String> {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()).map(|s| s.to_string()));

    ip.and_then(|ip| auth::hash_ip(&state.order_token_hmac_secret, &ip).ok())
}

pub async fn waitlist_join(
    State(state): State<HttpState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<WaitlistJoinRequest>,
) -> Result<Response, ApiError> {
    let email = payload.email.trim().to_lowercase();
    validate_email(&email)?;

    if let Some(existing) = db::find_waitlist_entry_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?
    {
        let position = db::get_waitlist_position(&state.pool, existing.id)
            .await
            .map_err(internal_err)?;
        let body = WaitlistJoinResponse {
            referral_code: existing.referral_code,
            position,
        };
        let json = serde_json::to_string(&body).unwrap_or_default();
        return Ok(
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], json)
                .into_response(),
        );
    }

    let mut referral_code = generate_referral_code();
    for _ in 0..5 {
        if !db::referral_code_exists(&state.pool, &referral_code)
            .await
            .map_err(internal_err)?
        {
            break;
        }
        referral_code = generate_referral_code();
    }

    let referred_by = if let Some(ref_code) = &payload.ref_code {
        let entry = db::find_waitlist_entry_by_referral_code(&state.pool, ref_code)
            .await
            .map_err(internal_err)?;
        if entry.is_some() {
            Some(ref_code.clone())
        } else {
            None
        }
    } else {
        None
    };

    let ip_hash = extract_ip_hash(&state, &headers);

    let entry = db::create_waitlist_entry(
        &state.pool,
        &email,
        payload.display_name.as_deref(),
        &referral_code,
        referred_by.as_deref(),
        ip_hash.as_deref(),
        payload.utm_source.as_deref(),
        payload.utm_medium.as_deref(),
        payload.utm_campaign.as_deref(),
        payload.utm_content.as_deref(),
        payload.utm_term.as_deref(),
    )
    .await
    .map_err(internal_err)?;

    let position = db::get_waitlist_position(&state.pool, entry.id)
        .await
        .map_err(internal_err)?;

    let verification_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&verification_token).map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::create_waitlist_verify_token(&state.pool, entry.id, &token_hash, expires_at)
        .await
        .map_err(internal_err)?;

    if let Some(ref email_client) = state.email_client {
        let verification_link = format!(
            "{}/waitlist/verify?token={}",
            state.app_base_url.trim_end_matches('/'),
            verification_token
        );
        if let Err(e) = email_client
            .send_waitlist_verification_email(&email, &verification_link)
            .await
        {
            tracing::warn!(error = %e, "failed to send waitlist verification email");
        }
    } else {
        tracing::warn!(
            "no email client configured — waitlist verification token for {}: {}",
            email,
            verification_token
        );
    }

    let body = WaitlistJoinResponse {
        referral_code,
        position,
    };
    let json = serde_json::to_string(&body).unwrap_or_default();
    Ok(
        (StatusCode::CREATED, [(axum::http::header::CONTENT_TYPE, "application/json")], json)
            .into_response(),
    )
}

pub async fn waitlist_verify(
    State(state): State<HttpState>,
    Json(payload): Json<WaitlistVerifyRequest>,
) -> Result<Json<WaitlistVerifyResponse>, ApiError> {
    let token_hash = auth::hash_verification_token(&payload.token).map_err(internal_err)?;

    let token_record = db::find_waitlist_verify_token(&state.pool, &token_hash)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("invalid or expired verification link"))?;

    db::verify_waitlist_email(&state.pool, token_record.entry_id)
        .await
        .map_err(internal_err)?;

    db::mark_waitlist_verify_token_used(&state.pool, token_record.id)
        .await
        .map_err(internal_err)?;

    let position = db::get_waitlist_position(&state.pool, token_record.entry_id)
        .await
        .map_err(internal_err)?;

    let row = sqlx::query(
        "SELECT email, referral_code FROM waitlist_entries WHERE id = $1",
    )
    .bind(token_record.entry_id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_err)?;

    Ok(Json(WaitlistVerifyResponse {
        email: row.try_get("email").map_err(internal_err)?,
        position,
        referral_code: row.try_get("referral_code").map_err(internal_err)?,
    }))
}

pub async fn waitlist_resend(
    State(state): State<HttpState>,
    Json(payload): Json<WaitlistResendRequest>,
) -> Result<StatusCode, ApiError> {
    let email = payload.email.trim().to_lowercase();
    validate_email(&email)?;

    let entry = db::find_waitlist_entry_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("email not found on waitlist"))?;

    if entry.email_verified {
        return Err(ApiError::bad_request("email already verified"));
    }

    let verification_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&verification_token).map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::create_waitlist_verify_token(&state.pool, entry.id, &token_hash, expires_at)
        .await
        .map_err(internal_err)?;

    if let Some(ref email_client) = state.email_client {
        let verification_link = format!(
            "{}/waitlist/verify?token={}",
            state.app_base_url.trim_end_matches('/'),
            verification_token
        );
        if let Err(e) = email_client
            .send_waitlist_verification_email(&email, &verification_link)
            .await
        {
            tracing::warn!(error = %e, "failed to resend waitlist verification email");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn waitlist_stats(
    State(state): State<HttpState>,
) -> Result<Json<WaitlistStatsResponse>, ApiError> {
    let (total, verified) = db::count_waitlist_entries(&state.pool)
        .await
        .map_err(internal_err)?;

    Ok(Json(WaitlistStatsResponse { total, verified }))
}
