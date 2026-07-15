use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::SameSite;
use chrono::{Duration, Utc};
use secrecy::SecretString;
use uuid::Uuid;

use crate::{
    db::{self, UserRow},
    http::auth,
    http::error::ApiError,
    http::handlers::HttpState,
    http::mw::AuthenticatedUser,
    http::types::{
        AuthUserResponse, ChangePasswordRequest, DeleteAccountRequest, ForgotPasswordRequest,
        LoginRequest, OrderHistoryItem, RegisterRequest, ResendVerificationRequest,
        ResetPasswordRequest, UpdateProfileRequest, VerifyEmailRequest,
    },
};

fn internal_err(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

fn set_auth_cookies(
    jar: CookieJar,
    access_token: &str,
    refresh_token: &str,
    csrf_token: &str,
    cookie_domain: Option<&str>,
) -> CookieJar {
    let secure = cfg!(not(debug_assertions));
    let same_site = if secure {
        SameSite::Strict
    } else {
        SameSite::Lax
    };

    let mut builder = axum_extra::extract::cookie::Cookie::build(("access_token", access_token.to_owned()))
        .http_only(true)
        .secure(secure)
        .same_site(same_site)
        .path("/api")
        .max_age(time::Duration::seconds(900));
    if let Some(domain) = cookie_domain {
        builder = builder.domain(domain.to_owned());
    }
    let jar = jar.add(builder.finish());

    let mut builder = axum_extra::extract::cookie::Cookie::build(("refresh_token", refresh_token.to_owned()))
        .http_only(true)
        .secure(secure)
        .same_site(same_site)
        .path("/api/v1/auth/refresh")
        .max_age(time::Duration::seconds(86400));
    if let Some(domain) = cookie_domain {
        builder = builder.domain(domain.to_owned());
    }
    let jar = jar.add(builder.finish());

    let mut builder = axum_extra::extract::cookie::Cookie::build(("csrf_token", csrf_token.to_owned()))
        .secure(secure)
        .same_site(same_site)
        .path("/")
        .max_age(time::Duration::seconds(900));
    if let Some(domain) = cookie_domain {
        builder = builder.domain(domain.to_owned());
    }
    jar.add(builder.finish())
}

fn cookie_domain_from_base_url(base_url: &str) -> Option<String> {
    let host = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
        .unwrap_or(base_url)
        .split(':')
        .next()
        .unwrap_or("");
    if host == "localhost" || host == "127.0.0.1" {
        return None;
    }
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        let domain = parts[parts.len() - 2..].join(".");
        Some(domain)
    } else {
        None
    }
}

fn clear_auth_cookies(jar: CookieJar, cookie_domain: Option<&str>) -> CookieJar {
    let mut c1 = axum_extra::extract::cookie::Cookie::build(("access_token", ""))
        .path("/api");
    let mut c2 = axum_extra::extract::cookie::Cookie::build(("refresh_token", ""))
        .path("/api/v1/auth/refresh");
    let mut c3 = axum_extra::extract::cookie::Cookie::build(("csrf_token", ""))
        .path("/");
    if let Some(domain) = cookie_domain {
        c1 = c1.domain(domain.to_owned());
        c2 = c2.domain(domain.to_owned());
        c3 = c3.domain(domain.to_owned());
    }
    let jar = jar.remove(c1.build());
    let jar = jar.remove(c2.build());
    jar.remove(c3.build())
}

fn user_to_response(user: &UserRow) -> AuthUserResponse {
    AuthUserResponse {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        email_verified: user.email_verified,
    }
}

fn auth_ok(jar: CookieJar, user: &UserRow) -> Response {
    auth_response(jar, StatusCode::OK, &user_to_response(user))
}

fn auth_created(jar: CookieJar, user: &UserRow) -> Response {
    auth_response(jar, StatusCode::CREATED, &user_to_response(user))
}

fn auth_response(jar: CookieJar, status: StatusCode, body: &AuthUserResponse) -> Response {
    let json = serde_json::to_string(body).unwrap_or_default();
    let mut response = (status, [(axum::http::header::CONTENT_TYPE, "application/json")], json).into_response();
    for cookie in jar.iter() {
        if let Ok(val) = cookie.encoded().to_string().parse() {
            response.headers_mut().append(axum::http::header::SET_COOKIE, val);
        }
    }
    response
}

async fn create_session(
    pool: &sqlx::PgPool,
    user: &UserRow,
    jwt_secret: &SecretString,
    access_token_ttl_minutes: i64,
    refresh_token_ttl_hours: i64,
) -> Result<(String, Uuid, String), ApiError> {
    let csrf_token = auth::generate_csrf_token();
    let access_jwt = auth::create_access_jwt(
        user.id,
        &user.email,
        &csrf_token,
        jwt_secret,
        access_token_ttl_minutes,
    )
    .map_err(internal_err)?;

    let refresh_id = Uuid::now_v7();
    let expires_at = Utc::now() + Duration::hours(refresh_token_ttl_hours);

    db::create_refresh_token(pool, refresh_id, user.id, expires_at, None, None)
        .await
        .map_err(internal_err)?;

    Ok((access_jwt, refresh_id, csrf_token))
}

pub async fn register(
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(payload): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    let email = payload.email.trim().to_lowercase();
    if !auth::is_valid_email(&email) {
        return Err(ApiError::bad_request("invalid email address"));
    }
    if payload.password.len() < 8 {
        return Err(ApiError::bad_request("password must be at least 8 characters"));
    }

    let existing = db::find_user_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?;
    if existing.is_some() {
        return Err(ApiError::conflict("an account with this email already exists"));
    }

    let password_hash =
        auth::hash_password(&payload.password).map_err(internal_err)?;

    let user = db::create_user(
        &state.pool,
        &email,
        payload.display_name.as_deref(),
        &password_hash,
    )
    .await
    .map_err(internal_err)?;

    let verification_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&verification_token)
        .map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::create_email_token(&state.pool, user.id, &token_hash, "verify_email", expires_at)
        .await
        .map_err(internal_err)?;

    if let Some(ref email_client) = state.email_client {
        let verification_link = format!(
            "{}/verify?token={}",
            state.app_base_url.trim_end_matches('/'),
            verification_token
        );
        if let Err(e) = email_client
            .send_verification_email(&email, &verification_link)
            .await
        {
            tracing::warn!(error = %e, "failed to send verification email");
        }
    } else {
        tracing::warn!(
            "no email client configured — verification token for {}: {}",
            email,
            verification_token
        );
    }

    let (access_jwt, refresh_id, csrf_token) = create_session(
        &state.pool,
        &user,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
        state.refresh_token_ttl_hours,
    )
    .await?;

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = set_auth_cookies(jar, &access_jwt, &refresh_id.to_string(), &csrf_token, cookie_domain.as_deref());

    Ok(auth_created(jar, &user))
}

pub async fn login(
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let email = payload.email.trim().to_lowercase();

    let user = db::find_user_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("invalid email or password"))?;

    let valid = auth::verify_password(&payload.password, &user.password_hash)
        .map_err(internal_err)?;

    if !valid {
        return Err(ApiError::unauthorized("invalid email or password"));
    }

    if !user.email_verified {
        return Err(ApiError::forbidden("email_not_verified"));
    }

    let (access_jwt, refresh_id, csrf_token) = create_session(
        &state.pool,
        &user,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
        state.refresh_token_ttl_hours,
    )
    .await?;

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = set_auth_cookies(jar, &access_jwt, &refresh_id.to_string(), &csrf_token, cookie_domain.as_deref());

    Ok(auth_ok(jar, &user))
}

pub async fn refresh(
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Response, ApiError> {
    let refresh_token_str = jar
        .get("refresh_token")
        .map(|c| c.value())
        .unwrap_or("");

    let refresh_id: Uuid = refresh_token_str
        .parse()
        .map_err(|_| ApiError::unauthorized("invalid refresh token"))?;

    let stored = db::find_refresh_token(&state.pool, refresh_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("invalid refresh token"))?;

    if stored.revoked_at.is_some() {
        db::revoke_all_user_refresh_tokens(&state.pool, stored.user_id)
            .await
            .map_err(internal_err)?;
        return Err(ApiError::unauthorized("refresh token revoked"));
    }

    if stored.expires_at < Utc::now() {
        return Err(ApiError::unauthorized("refresh token expired"));
    }

    db::revoke_refresh_token(&state.pool, refresh_id)
        .await
        .map_err(internal_err)?;

    let user = db::find_user_by_id(&state.pool, stored.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| internal_err("user not found"))?;

    let (access_jwt, new_refresh_id, csrf_token) = create_session(
        &state.pool,
        &user,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
        state.refresh_token_ttl_hours,
    )
    .await?;

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = set_auth_cookies(jar, &access_jwt, &new_refresh_id.to_string(), &csrf_token, cookie_domain.as_deref());

    Ok(auth_ok(jar, &user))
}

pub async fn logout(
    State(state): State<HttpState>,
    Extension(_user): Extension<AuthenticatedUser>,
    jar: CookieJar,
) -> Result<Response, ApiError> {
    if let Some(refresh_cookie) = jar.get("refresh_token") {
        if let Ok(refresh_id) = refresh_cookie.value().parse::<Uuid>() {
            let _ = db::revoke_refresh_token(&state.pool, refresh_id).await;
        }
    }

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = clear_auth_cookies(jar, cookie_domain.as_deref());
    let mut response = StatusCode::NO_CONTENT.into_response();
    for cookie in jar.iter() {
        if let Ok(val) = cookie.encoded().to_string().parse() {
            response.headers_mut().append(axum::http::header::SET_COOKIE, val);
        }
    }
    Ok(response)
}

pub async fn get_me(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<AuthUserResponse>, ApiError> {
    let db_user = db::find_user_by_id(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    Ok(Json(user_to_response(&db_user)))
}

pub async fn verify_email(
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<Response, ApiError> {
    let token_hash = auth::hash_verification_token(&payload.token)
        .map_err(internal_err)?;

    let email_token = db::find_email_token_by_hash(&state.pool, &token_hash, "verify_email")
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("invalid or expired verification token"))?;

    let user = db::find_user_by_id(&state.pool, email_token.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| internal_err("user not found"))?;

    db::verify_user_email(&state.pool, user.id)
        .await
        .map_err(internal_err)?;

    db::mark_email_token_used(&state.pool, email_token.id)
        .await
        .map_err(internal_err)?;

    let (access_jwt, refresh_id, csrf_token) = create_session(
        &state.pool,
        &user,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
        state.refresh_token_ttl_hours,
    )
    .await?;

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = set_auth_cookies(jar, &access_jwt, &refresh_id.to_string(), &csrf_token, cookie_domain.as_deref());

    Ok(auth_ok(jar, &user))
}

pub async fn resend_verification(
    State(state): State<HttpState>,
    Json(payload): Json<ResendVerificationRequest>,
) -> Result<StatusCode, ApiError> {
    let email = payload.email.trim().to_lowercase();

    let user = match db::find_user_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?
    {
        Some(u) => u,
        None => return Ok(StatusCode::OK),
    };

    if user.email_verified {
        return Ok(StatusCode::OK);
    }

    let verification_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&verification_token)
        .map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::create_email_token(&state.pool, user.id, &token_hash, "verify_email", expires_at)
        .await
        .map_err(internal_err)?;

    if let Some(ref email_client) = state.email_client {
        let verification_link = format!(
            "{}/verify?token={}",
            state.app_base_url.trim_end_matches('/'),
            verification_token
        );
        if let Err(e) = email_client
            .send_verification_email(&email, &verification_link)
            .await
        {
            tracing::warn!(error = %e, "failed to resend verification email");
        }
    }

    Ok(StatusCode::OK)
}

pub async fn forgot_password(
    State(state): State<HttpState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    let email = payload.email.trim().to_lowercase();

    let user = match db::find_user_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?
    {
        Some(u) => u,
        None => return Ok(StatusCode::OK),
    };

    let reset_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&reset_token)
        .map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(1);

    db::create_email_token(&state.pool, user.id, &token_hash, "reset_password", expires_at)
        .await
        .map_err(internal_err)?;

    if let Some(ref email_client) = state.email_client {
        let reset_link = format!(
            "{}/reset-password?token={}",
            state.app_base_url.trim_end_matches('/'),
            reset_token
        );
        if let Err(e) = email_client
            .send_password_reset_email(&email, &reset_link)
            .await
        {
            tracing::warn!(error = %e, "failed to send password reset email");
        }
    }

    Ok(StatusCode::OK)
}

pub async fn reset_password(
    State(state): State<HttpState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<StatusCode, ApiError> {
    if payload.password.len() < 8 {
        return Err(ApiError::bad_request("password must be at least 8 characters"));
    }

    let token_hash = auth::hash_verification_token(&payload.token)
        .map_err(internal_err)?;

    let email_token = db::find_email_token_by_hash(&state.pool, &token_hash, "reset_password")
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("invalid or expired reset token"))?;

    let password_hash = auth::hash_password(&payload.password).map_err(internal_err)?;

    db::update_user_password(&state.pool, email_token.user_id, &password_hash)
        .await
        .map_err(internal_err)?;

    db::mark_email_token_used(&state.pool, email_token.id)
        .await
        .map_err(internal_err)?;

    db::revoke_all_user_refresh_tokens(&state.pool, email_token.user_id)
        .await
        .map_err(internal_err)?;

    Ok(StatusCode::OK)
}

pub async fn get_order_history(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<OrderHistoryItem>>, ApiError> {
    let orders = db::find_orders_by_user(&state.pool, user.user_id, 50, 0)
        .await
        .map_err(internal_err)?;

    let items: Vec<OrderHistoryItem> = orders
        .into_iter()
        .map(|o| OrderHistoryItem {
            order_id: o.id,
            utility_slug: o.utility_slug,
            utility_type: o.utility_type,
            amount_ngn: o.amount_ngn,
            zec_amount: o.zec_amount.to_string(),
            status: o.status,
            created_at: o.created_at,
            completed_at: o.completed_at,
        })
        .collect();

    Ok(Json(items))
}

pub async fn update_profile(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<AuthUserResponse>, ApiError> {
    if let Some(ref name) = payload.display_name {
        if name.trim().is_empty() {
            return Err(ApiError::bad_request("display name cannot be empty"));
        }
        if name.len() > 100 {
            return Err(ApiError::bad_request("display name must be 100 characters or less"));
        }
    }

    let display_name = payload.display_name.as_deref().map(|n| n.trim());

    db::update_user_display_name(&state.pool, user.user_id, display_name)
        .await
        .map_err(internal_err)?;

    let db_user = db::find_user_by_id(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    Ok(Json(user_to_response(&db_user)))
}

pub async fn change_password(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    if payload.new_password.len() < 8 {
        return Err(ApiError::bad_request("new password must be at least 8 characters"));
    }

    let db_user = db::find_user_by_id(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    let valid = auth::verify_password(&payload.current_password, &db_user.password_hash)
        .map_err(internal_err)?;

    if !valid {
        return Err(ApiError::bad_request("current password is incorrect"));
    }

    let new_hash = auth::hash_password(&payload.new_password).map_err(internal_err)?;

    db::update_user_password(&state.pool, user.user_id, &new_hash)
        .await
        .map_err(internal_err)?;

    db::revoke_all_user_refresh_tokens(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    Ok(StatusCode::OK)
}

pub async fn delete_account(
    State(state): State<HttpState>,
    Extension(user): Extension<AuthenticatedUser>,
    jar: CookieJar,
    Json(payload): Json<DeleteAccountRequest>,
) -> Result<Response, ApiError> {
    let db_user = db::find_user_by_id(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    let valid = auth::verify_password(&payload.password, &db_user.password_hash)
        .map_err(internal_err)?;

    if !valid {
        return Err(ApiError::bad_request("password is incorrect"));
    }

    db::revoke_all_user_refresh_tokens(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    db::soft_delete_user(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    let cookie_domain = cookie_domain_from_base_url(&state.app_base_url);
    let jar = clear_auth_cookies(jar, cookie_domain.as_deref());
    let mut response = StatusCode::OK.into_response();
    for cookie in jar.iter() {
        if let Ok(val) = cookie.encoded().to_string().parse() {
            response.headers_mut().append(axum::http::header::SET_COOKIE, val);
        }
    }
    Ok(response)
}
