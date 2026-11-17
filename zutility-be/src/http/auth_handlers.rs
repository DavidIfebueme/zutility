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
    domain::money::CurrencyCode,
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

#[derive(Clone, Copy)]
enum AuthCookie {
    Access,
    Refresh,
    Csrf,
    Session,
}

impl AuthCookie {
    const ALL: [Self; 4] = [Self::Access, Self::Refresh, Self::Csrf, Self::Session];

    fn name(self) -> &'static str {
        match self {
            Self::Access => "access_token",
            Self::Refresh => "refresh_token",
            Self::Csrf => "csrf_token",
            Self::Session => "session",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Access => "/api",
            Self::Refresh => "/api/v1/auth",
            Self::Csrf | Self::Session => "/",
        }
    }

    fn http_only(self) -> bool {
        !matches!(self, Self::Csrf)
    }

    fn same_site(self) -> SameSite {
        match self {
            Self::Access | Self::Refresh => SameSite::Strict,
            Self::Csrf | Self::Session => SameSite::Lax,
        }
    }

    fn max_age(self, state: &HttpState) -> i64 {
        match self {
            Self::Access | Self::Csrf => access_ttl_seconds(state),
            Self::Refresh | Self::Session => refresh_ttl_seconds(state),
        }
    }
}

fn access_ttl_seconds(state: &HttpState) -> i64 {
    state.access_token_ttl_minutes.saturating_mul(60)
}

fn refresh_ttl_seconds(state: &HttpState) -> i64 {
    state.refresh_token_ttl_hours.saturating_mul(3600)
}

fn auth_cookie_domain(state: &HttpState) -> Option<String> {
    state
        .cookie_domain
        .clone()
        .or_else(|| cookie_domain_from_base_url(&state.app_base_url))
}

fn build_auth_cookie(
    cookie: AuthCookie,
    value: String,
    max_age: i64,
    cookie_domain: Option<&str>,
) -> axum_extra::extract::cookie::Cookie<'static> {
    let mut builder = axum_extra::extract::cookie::Cookie::build((cookie.name(), value))
        .http_only(cookie.http_only())
        .secure(cfg!(not(debug_assertions)))
        .same_site(cookie.same_site())
        .path(cookie.path())
        .max_age(time::Duration::seconds(max_age));
    if let Some(domain) = cookie_domain {
        builder = builder.domain(domain.to_owned());
    }
    builder.build()
}

fn set_auth_cookies(
    jar: CookieJar,
    state: &HttpState,
    access_token: &str,
    refresh_token: &str,
    csrf_token: &str,
) -> CookieJar {
    let domain = auth_cookie_domain(state);
    let domain = domain.as_deref();
    let values = [
        (AuthCookie::Access, access_token.to_owned()),
        (AuthCookie::Refresh, refresh_token.to_owned()),
        (AuthCookie::Csrf, csrf_token.to_owned()),
        (AuthCookie::Session, String::from("1")),
    ];

    let mut jar = jar;
    for (cookie, value) in values {
        jar = jar.add(build_auth_cookie(
            cookie,
            value,
            cookie.max_age(state),
            domain,
        ));
    }
    jar
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

fn clear_legacy_refresh_cookie(
    cookie_domain: Option<&str>,
) -> axum_extra::extract::cookie::Cookie<'static> {
    let mut builder = axum_extra::extract::cookie::Cookie::build(("refresh_token", String::new()))
        .http_only(true)
        .secure(cfg!(not(debug_assertions)))
        .same_site(SameSite::Lax)
        .path("/api/v1/auth/refresh")
        .max_age(time::Duration::seconds(0));
    if let Some(domain) = cookie_domain {
        builder = builder.domain(domain.to_owned());
    }
    builder.build()
}

fn clear_auth_cookies(state: &HttpState) -> Vec<axum_extra::extract::cookie::Cookie<'static>> {
    let domain = auth_cookie_domain(state);
    let domain = domain.as_deref();

    let mut cookies: Vec<_> = AuthCookie::ALL
        .iter()
        .map(|cookie| build_auth_cookie(*cookie, String::new(), 0, domain))
        .collect();
    cookies.push(clear_legacy_refresh_cookie(domain));
    cookies
}

fn with_cookie_list(
    mut response: Response,
    cookies: Vec<axum_extra::extract::cookie::Cookie<'static>>,
) -> Response {
    for cookie in cookies {
        if let Ok(value) = cookie.encoded().to_string().parse() {
            response
                .headers_mut()
                .append(axum::http::header::SET_COOKIE, value);
        }
    }
    response
}

fn record_auth_event(state: &HttpState, event: &str) {
    state.observability.metrics().record_auth_event(event);
    tracing::info!(auth_event = event, "auth event");
}

fn with_auth_cookies(mut response: Response, jar: CookieJar) -> Response {
    for cookie in jar.iter() {
        if let Ok(value) = cookie.encoded().to_string().parse() {
            response
                .headers_mut()
                .append(axum::http::header::SET_COOKIE, value);
        }
    }
    response
}

fn user_to_response(user: &UserRow) -> AuthUserResponse {
    AuthUserResponse {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        email_verified: user.email_verified,
        preferred_currency: user.preferred_currency.clone(),
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
    let response = (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response();
    with_auth_cookies(response, jar)
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

    db::create_refresh_token(
        pool, refresh_id, user.id, refresh_id, expires_at, None, None,
    )
    .await
    .map_err(internal_err)?;

    Ok((access_jwt, refresh_id, csrf_token))
}

fn mint_access_token(state: &HttpState, user: &UserRow) -> Result<(String, String), ApiError> {
    let csrf_token = auth::generate_csrf_token();
    let access_jwt = auth::create_access_jwt(
        user.id,
        &user.email,
        &csrf_token,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
    )
    .map_err(internal_err)?;
    Ok((access_jwt, csrf_token))
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
        return Err(ApiError::bad_request(
            "password must be at least 8 characters",
        ));
    }

    let existing = db::find_user_by_email(&state.pool, &email)
        .await
        .map_err(internal_err)?;
    if existing.is_some() {
        return Err(ApiError::conflict(
            "an account with this email already exists",
        ));
    }

    let password_hash = auth::hash_password(&payload.password).map_err(internal_err)?;

    let preferred_currency = match payload.preferred_currency.as_deref() {
        None => CurrencyCode::Ngn,
        Some(raw) => CurrencyCode::from_iso(raw)
            .ok_or_else(|| ApiError::bad_request("unsupported currency"))?,
    };

    let user = db::create_user(
        &state.pool,
        &email,
        payload.display_name.as_deref(),
        &password_hash,
        preferred_currency.iso(),
    )
    .await
    .map_err(internal_err)?;

    let verification_token = auth::generate_verification_token();
    let token_hash = auth::hash_verification_token(&verification_token).map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::invalidate_email_tokens(&state.pool, user.id, "verify_email")
        .await
        .map_err(internal_err)?;
    db::create_email_token(
        &state.pool,
        user.id,
        &token_hash,
        "verify_email",
        expires_at,
    )
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

    let jar = set_auth_cookies(
        jar,
        &state,
        &access_jwt,
        &refresh_id.to_string(),
        &csrf_token,
    );

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

    if let Some(locked_until) = db::login_lockout_until(&state.pool, user.id)
        .await
        .map_err(internal_err)?
        && locked_until > Utc::now()
    {
        return Err(ApiError::too_many_requests(
            "too many failed attempts, try again in a few minutes",
        ));
    }

    let valid =
        auth::verify_password(&payload.password, &user.password_hash).map_err(internal_err)?;

    if !valid {
        let _ = db::record_failed_login(&state.pool, user.id).await;
        return Err(ApiError::unauthorized("invalid email or password"));
    }

    let _ = db::clear_failed_logins(&state.pool, user.id).await;

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

    let jar = set_auth_cookies(
        jar,
        &state,
        &access_jwt,
        &refresh_id.to_string(),
        &csrf_token,
    );

    Ok(auth_ok(jar, &user))
}

const REFRESH_REUSE_GRACE_SECONDS: i64 = 900;

pub async fn refresh(State(state): State<HttpState>, jar: CookieJar) -> Result<Response, ApiError> {
    let refresh_token_str = jar.get("refresh_token").map(|c| c.value()).unwrap_or("");

    let refresh_id: Uuid = refresh_token_str
        .parse()
        .map_err(|_| ApiError::unauthorized("invalid refresh token"))?;

    let mut tx = db::begin_tx(&state.pool).await.map_err(internal_err)?;
    let owner = db::refresh_token_owner_tx(&mut tx, refresh_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("invalid refresh token"))?;
    db::lock_user_sessions_tx(&mut tx, owner)
        .await
        .map_err(internal_err)?;
    let claimed = db::claim_refresh_token_tx(&mut tx, refresh_id)
        .await
        .map_err(internal_err)?;

    if let Some(claimed) = claimed {
        if claimed.expires_at < Utc::now() {
            record_auth_event(&state, "refresh_expired");
            return Err(ApiError::unauthorized("refresh token expired"));
        }

        let user = db::find_user_by_id_tx(&mut tx, claimed.user_id)
            .await
            .map_err(internal_err)?
            .ok_or_else(|| ApiError::unauthorized("session ended"))?;

        let new_refresh_id = Uuid::now_v7();
        let expires_at = Utc::now() + Duration::hours(state.refresh_token_ttl_hours);
        db::insert_refresh_token_tx(
            &mut tx,
            new_refresh_id,
            user.id,
            claimed.family_id,
            expires_at,
            None,
            None,
        )
        .await
        .map_err(internal_err)?;
        db::link_refresh_token_replacement_tx(&mut tx, refresh_id, new_refresh_id)
            .await
            .map_err(internal_err)?;
        tx.commit().await.map_err(internal_err)?;
        record_auth_event(&state, "refresh_rotated");

        let (access_jwt, csrf_token) = mint_access_token(&state, &user)?;
        let jar = set_auth_cookies(
            jar,
            &state,
            &access_jwt,
            &new_refresh_id.to_string(),
            &csrf_token,
        );
        return Ok(auth_ok(jar, &user));
    }

    tx.rollback().await.map_err(internal_err)?;

    let stored = db::find_refresh_token(&state.pool, refresh_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("invalid refresh token"))?;

    if stored.expires_at < Utc::now() {
        record_auth_event(&state, "refresh_expired");
        return Err(ApiError::unauthorized("refresh token expired"));
    }

    let Some(revoked_at) = stored.revoked_at else {
        return Err(ApiError::service_unavailable(
            "refresh token is being rotated, try again",
        ));
    };

    let rotated_recently = stored.replaced_by.is_some()
        && Utc::now() - revoked_at <= Duration::seconds(REFRESH_REUSE_GRACE_SECONDS);

    if rotated_recently {
        let user = db::find_user_by_id(&state.pool, stored.user_id)
            .await
            .map_err(internal_err)?
            .ok_or_else(|| ApiError::unauthorized("session ended"))?;

        if let Some(successor) = db::find_live_successor(&state.pool, stored.id)
            .await
            .map_err(internal_err)?
        {
            record_auth_event(&state, "refresh_replay");
            let (access_jwt, csrf_token) = mint_access_token(&state, &user)?;
            let jar = set_auth_cookies(
                jar,
                &state,
                &access_jwt,
                &successor.to_string(),
                &csrf_token,
            );
            return Ok(auth_ok(jar, &user));
        }

        return Err(ApiError::unauthorized("refresh token already rotated"));
    }

    if stored.replaced_by.is_some() {
        db::revoke_refresh_token_family(&state.pool, stored.id)
            .await
            .map_err(internal_err)?;
        record_auth_event(&state, "refresh_family_revoked");
    }
    record_auth_event(&state, "refresh_rejected");
    Err(ApiError::unauthorized("refresh token already rotated"))
}

pub async fn logout(State(state): State<HttpState>, jar: CookieJar) -> Result<Response, ApiError> {
    if let Some(refresh_cookie) = jar.get("refresh_token")
        && let Ok(refresh_id) = refresh_cookie.value().parse::<Uuid>()
    {
        db::revoke_refresh_token_family(&state.pool, refresh_id)
            .await
            .map_err(internal_err)?;
    }

    record_auth_event(&state, "logout");
    let cookies = clear_auth_cookies(&state);
    Ok(with_cookie_list(
        StatusCode::NO_CONTENT.into_response(),
        cookies,
    ))
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
    let token_hash = auth::hash_verification_token(&payload.token).map_err(internal_err)?;

    let email_token = db::find_email_token_by_hash(&state.pool, &token_hash, "verify_email")
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("invalid or expired verification token"))?;

    if !db::mark_email_token_used(&state.pool, email_token.id)
        .await
        .map_err(internal_err)?
    {
        return Err(ApiError::bad_request(
            "invalid or expired verification token",
        ));
    }

    let mut user = db::find_user_by_id(&state.pool, email_token.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| internal_err("user not found"))?;

    if user.email_verified {
        return Err(ApiError::bad_request("email is already verified"));
    }

    db::verify_user_email(&state.pool, user.id)
        .await
        .map_err(internal_err)?;
    user.email_verified = true;

    let (access_jwt, refresh_id, csrf_token) = create_session(
        &state.pool,
        &user,
        &state.jwt_secret,
        state.access_token_ttl_minutes,
        state.refresh_token_ttl_hours,
    )
    .await?;

    let jar = set_auth_cookies(
        jar,
        &state,
        &access_jwt,
        &refresh_id.to_string(),
        &csrf_token,
    );

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
    let token_hash = auth::hash_verification_token(&verification_token).map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(24);

    db::invalidate_email_tokens(&state.pool, user.id, "verify_email")
        .await
        .map_err(internal_err)?;
    db::create_email_token(
        &state.pool,
        user.id,
        &token_hash,
        "verify_email",
        expires_at,
    )
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
    let token_hash = auth::hash_verification_token(&reset_token).map_err(internal_err)?;
    let expires_at = Utc::now() + Duration::hours(1);

    db::create_email_token(
        &state.pool,
        user.id,
        &token_hash,
        "reset_password",
        expires_at,
    )
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
        return Err(ApiError::bad_request(
            "password must be at least 8 characters",
        ));
    }

    let token_hash = auth::hash_verification_token(&payload.token).map_err(internal_err)?;

    let email_token = db::find_email_token_by_hash(&state.pool, &token_hash, "reset_password")
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::bad_request("invalid or expired reset token"))?;

    if !db::mark_email_token_used(&state.pool, email_token.id)
        .await
        .map_err(internal_err)?
    {
        return Err(ApiError::bad_request("invalid or expired reset token"));
    }

    let password_hash = auth::hash_password(&payload.password).map_err(internal_err)?;

    db::update_user_password(&state.pool, email_token.user_id, &password_hash)
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
            return Err(ApiError::bad_request(
                "display name must be 100 characters or less",
            ));
        }
    }

    let display_name = payload.display_name.as_deref().map(|n| n.trim());

    let preferred_currency = match payload.preferred_currency.as_deref() {
        None => None,
        Some(raw) => Some(
            CurrencyCode::from_iso(raw)
                .ok_or_else(|| ApiError::bad_request("unsupported currency"))?
                .iso(),
        ),
    };

    db::update_user_display_name(&state.pool, user.user_id, display_name)
        .await
        .map_err(internal_err)?;

    if let Some(code) = preferred_currency {
        db::update_user_preferred_currency(&state.pool, user.user_id, code)
            .await
            .map_err(internal_err)?;
    }

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
        return Err(ApiError::bad_request(
            "new password must be at least 8 characters",
        ));
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
    _jar: CookieJar,
    Json(payload): Json<DeleteAccountRequest>,
) -> Result<Response, ApiError> {
    let db_user = db::find_user_by_id(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;

    let valid =
        auth::verify_password(&payload.password, &db_user.password_hash).map_err(internal_err)?;

    if !valid {
        return Err(ApiError::bad_request("password is incorrect"));
    }

    db::revoke_all_user_refresh_tokens(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    db::soft_delete_user(&state.pool, user.user_id)
        .await
        .map_err(internal_err)?;

    let cookies = clear_auth_cookies(&state);
    Ok(with_cookie_list(StatusCode::OK.into_response(), cookies))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_the_cookie_domain_from_the_public_host() {
        assert_eq!(
            cookie_domain_from_base_url("https://www.zutility.xyz").as_deref(),
            Some("zutility.xyz")
        );
        assert_eq!(
            cookie_domain_from_base_url("https://api.zutility.xyz").as_deref(),
            Some("zutility.xyz")
        );
        assert_eq!(cookie_domain_from_base_url("http://localhost:3000"), None);
        assert_eq!(cookie_domain_from_base_url("http://127.0.0.1:3001"), None);
    }

    #[test]
    fn refresh_cookie_covers_every_auth_path() {
        let cookie = build_auth_cookie(
            AuthCookie::Refresh,
            String::from("token"),
            604_800,
            Some("zutility.xyz"),
        );
        assert_eq!(cookie.path(), Some("/api/v1/auth"));
        assert_eq!(cookie.max_age(), Some(time::Duration::seconds(604_800)));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
        assert_eq!(cookie.domain(), Some("zutility.xyz"));
    }

    #[test]
    fn every_cookie_has_a_path_the_browser_can_reach_it_on() {
        assert_eq!(AuthCookie::Access.name(), "access_token");
        assert_eq!(AuthCookie::Refresh.name(), "refresh_token");
        assert_eq!(AuthCookie::Csrf.name(), "csrf_token");
        assert_eq!(AuthCookie::Session.name(), "session");
        assert_eq!(AuthCookie::Access.path(), "/api");
        assert_eq!(AuthCookie::Refresh.path(), "/api/v1/auth");
        assert_eq!(AuthCookie::Csrf.path(), "/");
        assert_eq!(AuthCookie::Session.path(), "/");
        assert!(!AuthCookie::Csrf.http_only());
        assert!(AuthCookie::Session.http_only());
        assert_eq!(AuthCookie::Csrf.same_site(), SameSite::Lax);
        assert_eq!(AuthCookie::Access.same_site(), SameSite::Strict);
    }

    #[test]
    fn logout_also_expires_the_cookie_from_the_old_path() {
        let cookie = clear_legacy_refresh_cookie(Some("zutility.xyz"));
        assert_eq!(cookie.name(), "refresh_token");
        assert_eq!(cookie.path(), Some("/api/v1/auth/refresh"));
        assert_eq!(cookie.max_age(), Some(time::Duration::seconds(0)));
    }

    #[test]
    fn clearing_a_cookie_expires_it_instead_of_letting_the_jar_drop_it() {
        let cookie = build_auth_cookie(AuthCookie::Refresh, String::new(), 0, Some("zutility.xyz"));
        assert_eq!(cookie.max_age(), Some(time::Duration::seconds(0)));
        assert_eq!(cookie.domain(), Some("zutility.xyz"));
    }
}
