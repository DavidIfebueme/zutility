use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, header};
use axum::middleware::{self, Next};
use secrecy::SecretString;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use uuid::Uuid;

use zutility_be::http;

static SEED: OnceCell<()> = OnceCell::const_new();

pub struct Session {
    pub jwt: String,
    pub csrf: String,
}

pub async fn pool() -> Option<PgPool> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("zutility_be=debug"))
        .try_init();
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url)
        .await
        .expect("TEST_DATABASE_URL is set but the test database is unreachable");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("test database migrations failed");
    SEED.get_or_init(|| seed_addresses(&pool)).await;
    Some(pool)
}

async fn seed_addresses(pool: &PgPool) {
    let prefix = format!("ztest-{}-", std::process::id());
    for index in 0..200 {
        let address = format!("{prefix}{index}");
        let _ = sqlx::query(
            "INSERT INTO deposit_addresses (address, address_type, network, used)
             VALUES ($1, 'shielded', 'testnet', false)
             ON CONFLICT (address) DO NOTHING",
        )
        .bind(address)
        .execute(pool)
        .await;
    }
}

#[allow(dead_code)]
pub async fn session(pool: &PgPool) -> Option<Session> {
    let email = format!("test-{}@example.com", Uuid::new_v4());
    let user = zutility_be::db::create_user(pool, &email, Some("probe"), "hash", "NGN")
        .await
        .expect("create test user");
    let csrf = zutility_be::http::auth::generate_csrf_token();
    let secret = SecretString::from(http::DEV_JWT_SECRET.to_string());
    let jwt = zutility_be::http::auth::create_access_jwt(user.id, &user.email, &csrf, &secret, 15)
        .expect("create test jwt");
    Some(Session { jwt, csrf })
}

#[allow(dead_code)]
pub async fn authed_router() -> Option<Router> {
    let pool = pool().await?;
    let session = session(&pool).await?;
    Some(authed_app(pool, &session))
}

pub fn authed_app(pool: PgPool, session: &Session) -> Router {
    let cookie = format!("access_token={}; csrf_token={}", session.jwt, session.csrf);
    let csrf = session.csrf.clone();
    http::router_with_pool(Some(pool)).layer(middleware::from_fn(
        move |mut request: Request, next: Next| {
            let cookie = cookie.clone();
            let csrf = csrf.clone();
            async move {
                if let Ok(value) = HeaderValue::from_str(&cookie) {
                    request.headers_mut().insert(header::COOKIE, value);
                }
                if let Ok(value) = HeaderValue::from_str(&csrf) {
                    request.headers_mut().insert("x-csrf-token", value);
                }
                next.run(request).await
            }
        },
    ))
}
