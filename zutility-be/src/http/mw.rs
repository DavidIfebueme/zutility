use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;
use secrecy::{ExposeSecret, SecretString};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use super::auth;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
}

pub async fn auth_middleware(
    cookie_jar: CookieJar,
    State(jwt_secret): State<SecretString>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let access_token = cookie_jar
        .get("access_token")
        .map(|c| c.value())
        .unwrap_or("");

    if access_token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims = auth::verify_access_jwt(access_token, &jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if request.method() != axum::http::Method::GET {
        let csrf_header = request
            .headers()
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let csrf_cookie = cookie_jar
            .get("csrf_token")
            .map(|c| c.value())
            .unwrap_or("");

        if csrf_header.is_empty()
            || csrf_cookie.is_empty()
            || csrf_header != csrf_cookie
            || csrf_header != claims.csrf
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = AuthenticatedUser {
        user_id,
        email: claims.email,
    };

    let mut request = request;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

pub async fn admin_middleware(
    State(admin_secret): State<Option<SecretString>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let secret = admin_secret.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let provided = request
        .headers()
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = secret.expose_secret();

    if provided.is_empty() || expected.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let provided_bytes: &[u8] = provided.as_bytes();
    let expected_bytes: &[u8] = expected.as_bytes();

    if provided_bytes.len() != expected_bytes.len()
        || !bool::from(provided_bytes.ct_eq(expected_bytes))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
