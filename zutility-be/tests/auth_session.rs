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
        .max_connections(8)
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
        refresh_id,
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

fn refresh_token_from(response: &axum::response::Response) -> Option<Uuid> {
    set_cookies(response)
        .iter()
        .find(|cookie| cookie.starts_with("refresh_token=") && !cookie.contains("Max-Age=0"))
        .and_then(|cookie| {
            cookie
                .split(';')
                .next()
                .and_then(|pair| pair.strip_prefix("refresh_token="))
                .and_then(|value| value.parse().ok())
        })
}

async fn seed_token(pool: &PgPool, user_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    db::create_refresh_token(
        pool,
        id,
        user_id,
        id,
        Utc::now() + Duration::hours(1),
        None,
        None,
    )
    .await
    .expect("token");
    id
}

async fn count_rows(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("count")
}

async fn count_active(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM refresh_tokens WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("active")
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

#[tokio::test]
async fn concurrent_refreshes_rotate_once_and_converge() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (user_id, refresh_id) = seed_user(&pool).await;
    let state = state(pool.clone());

    let mut handles = Vec::new();
    for _ in 0..8 {
        let state = state.clone();
        handles.push(tokio::spawn(async move {
            refresh(State(state), jar_with(refresh_id)).await.is_ok()
        }));
    }
    for handle in handles {
        assert!(handle.await.expect("task"), "every concurrent refresh succeeds");
    }

    assert_eq!(
        count_rows(&pool, user_id).await,
        2,
        "one rotation creates exactly one successor"
    );
    assert_eq!(
        count_active(&pool, user_id).await,
        1,
        "exactly one live token remains"
    );
}

#[tokio::test]
async fn a_lost_response_replays_into_the_live_successor() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (user_id, refresh_id) = seed_user(&pool).await;
    let state = state(pool.clone());

    let first = refresh(State(state.clone()), jar_with(refresh_id))
        .await
        .expect("first refresh rotates");
    let successor = refresh_token_from(&first).expect("successor token");

    let replay = refresh(State(state), jar_with(refresh_id))
        .await
        .expect("the same token replays inside the grace window");

    assert_eq!(
        refresh_token_from(&replay),
        Some(successor),
        "the retry receives the live successor, not a dead token"
    );
    assert_eq!(
        count_rows(&pool, user_id).await,
        2,
        "a replay does not mint another token"
    );
}

#[tokio::test]
async fn a_stale_replay_revokes_only_its_own_family() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (user_id, old_token) = seed_user(&pool).await;
    let other_device = seed_token(&pool, user_id).await;
    let state = state(pool.clone());

    refresh(State(state.clone()), jar_with(old_token))
        .await
        .expect("rotate the family first");

    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() - interval '1 hour' WHERE id = $1")
        .bind(old_token)
        .execute(&pool)
        .await
        .expect("age the token");

    let result = refresh(State(state), jar_with(old_token)).await;
    assert!(matches!(result, Err(ApiError::Unauthorized(_))));

    let family_alive = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM refresh_tokens
         WHERE user_id = $1 AND revoked_at IS NULL AND family_id = $2",
    )
    .bind(user_id)
    .bind(old_token)
    .fetch_one(&pool)
    .await
    .expect("family count");
    assert_eq!(family_alive, 0, "a stale replay revokes its own family");

    let other_alive = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NULL FROM refresh_tokens WHERE id = $1",
    )
    .bind(other_device)
    .fetch_one(&pool)
    .await
    .expect("other token");
    assert!(
        other_alive,
        "a stale replay must not sign out the other device"
    );
}

#[tokio::test]
async fn an_expired_token_does_not_rotate() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (user_id, refresh_id) = seed_user(&pool).await;
    let state = state(pool.clone());

    sqlx::query("UPDATE refresh_tokens SET expires_at = now() - interval '1 minute' WHERE id = $1")
        .bind(refresh_id)
        .execute(&pool)
        .await
        .expect("expire");

    let result = refresh(State(state), jar_with(refresh_id)).await;
    assert!(matches!(result, Err(ApiError::Unauthorized(_))));
    assert_eq!(count_rows(&pool, user_id).await, 1, "expiry mints nothing");
}

#[tokio::test]
async fn logout_and_refresh_cannot_both_win() {
    let Some(pool) = test_pool().await else {
        return;
    };

    for _ in 0..10 {
        let (user_id, token) = seed_user(&pool).await;
        let state = state(pool.clone());

        let rotating = tokio::spawn({
            let state = state.clone();
            async move {
                let _ = refresh(State(state), jar_with(token)).await;
            }
        });
        let signing_out = tokio::spawn({
            let state = state.clone();
            async move {
                let _ = logout(State(state), jar_with(token)).await;
            }
        });

        let _ = tokio::join!(rotating, signing_out);

        assert_eq!(
            count_active(&pool, user_id).await,
            0,
            "logout must leave no live token, even when a refresh runs at the same time"
        );
    }
}

#[tokio::test]
async fn email_token_mark_is_a_compare_and_set() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await else {
        return Ok(());
    };
    let (user_id, _) = seed_user(&pool).await;

    db::create_email_token(
        &pool,
        user_id,
        "verify-hash",
        "verify_email",
        Utc::now() + Duration::hours(1),
    )
    .await?;

    let token_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM email_verification_tokens WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    assert!(db::mark_email_token_used(&pool, token_id).await?);
    assert!(!db::mark_email_token_used(&pool, token_id).await?);

    Ok(())
}

#[tokio::test]
async fn five_failed_logins_lock_the_account() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await else {
        return Ok(());
    };
    let (user_id, _) = seed_user(&pool).await;

    for _ in 0..5 {
        db::record_failed_login(&pool, user_id).await?;
    }

    let locked = db::login_lockout_until(&pool, user_id).await?;
    assert!(locked.is_some(), "the fifth failure should lock the account");

    db::clear_failed_logins(&pool, user_id).await?;
    assert!(db::login_lockout_until(&pool, user_id).await?.is_none());

    Ok(())
}
