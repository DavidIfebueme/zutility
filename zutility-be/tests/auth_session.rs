use axum::extract::State;
use axum::http::{HeaderMap, header::SET_COOKIE};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use secrecy::SecretString;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use zutility_be::db;
use zutility_be::http::auth_handlers::{logout, refresh};
use zutility_be::http::error::ApiError;
use zutility_be::http::handlers::HttpState;

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(pool)
}

fn state(pool: PgPool) -> HttpState {
    let mut state = HttpState::new(
        SecretString::from(String::from("test_order_token_secret")),
        30,
        15,
        pool,
        SecretString::from(String::from("test_jwt_secret_for_sessions")),
        15,
        168,
        String::from("https://www.zutility.xyz"),
    );
    state.cookie_domain = Some(String::from("zutility.xyz"));
    state
}

async fn seed_user(pool: &PgPool) -> (Uuid, Uuid) {
    let email = format!("session-{}@zutility.xyz", Uuid::new_v4().simple());
    let user = db::create_user(pool, &email, Some("probe"), "not-a-real-hash")
        .await
        .expect("user");
    db::verify_user_email(pool, user.id)
        .await
        .expect("verified");

    let refresh_id = Uuid::now_v7();
    db::create_refresh_token(
        pool,
        refresh_id,
        user.id,
        Utc::now() + Duration::hours(1),
        None,
        None,
    )
    .await
    .expect("refresh row");

    (user.id, refresh_id)
}

fn jar_with(token: Uuid) -> CookieJar {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::COOKIE,
        format!("refresh_token={token}").parse().expect("cookie"),
    );
    CookieJar::from_headers(&headers)
}

fn set_cookies(response: &axum::response::Response) -> Vec<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .collect()
}

#[tokio::test]
async fn logout_revokes_the_presented_token_without_an_access_token() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (_, refresh_id) = seed_user(&pool).await;
    let state = state(pool.clone());

    let response = logout(State(state.clone()), jar_with(refresh_id))
        .await
        .expect("logout succeeds without an access token");

    let revoked = sqlx::query_scalar::<_, Option<chrono::DateTime<Utc>>>(
        "SELECT revoked_at FROM refresh_tokens WHERE id = $1",
    )
    .bind(refresh_id)
    .fetch_one(&pool)
    .await
    .expect("row");
    assert!(revoked.is_some(), "logout must revoke the refresh row");

    let cookies = set_cookies(&response);
    assert!(
        cookies.iter().any(|cookie| cookie.starts_with("refresh_token=;")
            && cookie.contains("Max-Age=0")
            && cookie.contains("Path=/api/v1/auth;")
            && cookie.contains("Domain=zutility.xyz")),
        "logout must expire the refresh cookie on the current path: {cookies:?}"
    );
    assert!(
        cookies.iter().any(|cookie| cookie.starts_with("refresh_token=;")
            && cookie.contains("Path=/api/v1/auth/refresh")),
        "logout must also expire the cookie left by the previous release: {cookies:?}"
    );
    assert!(
        cookies.iter().any(|cookie| cookie.starts_with("session=;")),
        "logout must clear the session marker: {cookies:?}"
    );

    let after = refresh(State(state), jar_with(refresh_id)).await;
    assert!(
        matches!(after, Err(ApiError::Unauthorized(_))),
        "a logged out token must not refresh"
    );
}

#[tokio::test]
async fn refresh_issues_a_new_session_for_a_live_token() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (_, refresh_id) = seed_user(&pool).await;
    let state = state(pool.clone());

    let response = refresh(State(state), jar_with(refresh_id))
        .await
        .expect("live token refreshes");

    let cookies = set_cookies(&response);
    assert!(
        cookies
            .iter()
            .any(|cookie| cookie.starts_with("session=1") && cookie.contains("Path=/")),
        "refresh must reissue the session marker: {cookies:?}"
    );
    assert!(
        cookies
            .iter()
            .any(|cookie| cookie.starts_with("refresh_token=") && !cookie.contains("Max-Age=0")),
        "refresh must issue a new refresh token: {cookies:?}"
    );
}
