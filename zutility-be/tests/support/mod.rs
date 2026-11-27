use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, header};
use axum::middleware::{self, Next};
use rust_decimal::Decimal;
use secrecy::SecretString;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use uuid::Uuid;

use zutility_be::config::{AppConfig, AppEnv, ZcashBackend, ZcashNetwork, ZcashRpcMode};
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
        sqlx::query(
            "INSERT INTO deposit_addresses (address, address_type, network, used)
             VALUES ($1, 'shielded', 'testnet', false)
             ON CONFLICT (address) DO NOTHING",
        )
        .bind(address)
        .execute(pool)
        .await
        .expect("seed test deposit addresses");
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
    let config = app_config(None, 1440);
    authed_app_with_config(pool, &config, session)
}

#[allow(dead_code)]
pub fn app_config(dtone_base_url: Option<&str>, catalogue_max_age_minutes: u32) -> AppConfig {
    let (dtone_api_key, dtone_api_secret) = match dtone_base_url {
        Some(_) => (
            Some(SecretString::from(String::from("dtone-key"))),
            Some(SecretString::from(String::from("dtone-secret"))),
        ),
        None => (None, None),
    };

    AppConfig {
        app_env: AppEnv::Dev,
        http_bind_addr: "127.0.0.1:3001".parse().expect("bind addr"),
        database_url: String::from("postgres://postgres:postgres@127.0.0.1:1/zutility"),
        order_token_hmac_secret: SecretString::from(String::from("dev_order_token_secret")),
        ip_hash_secret: SecretString::from(String::from("ip_secret")),
        vtpass_base_url: String::from("https://sandbox.vtpass.com/api"),
        vtpass_api_key: SecretString::from(String::from("key")),
        vtpass_secret_key: SecretString::from(String::from("secret")),
        remita_merchant_id: None,
        remita_api_key: None,
        remita_service_type_id: None,
        remita_base_url: None,
        remita_webhook_secret: None,
        remita_collections_base_url: None,
        remita_collections_secret_key: None,
        remita_collections_allow_demo: false,
        inlomax_api_key: None,
        inlomax_base_url: None,
        inlomax_webhook_secret: None,
        dtone_api_key,
        dtone_api_secret,
        dtone_base_url: dtone_base_url.map(ToOwned::to_owned),
        dtone_callback_url: Some(String::from(
            "https://api.zutility.test/api/v1/webhooks/dtone",
        )),
        dtone_callback_user: Some(SecretString::from(String::from("dtone-callback"))),
        dtone_callback_pass: Some(SecretString::from(String::from("dtone-callback-pass"))),
        dtone_callback_ips: Some(String::from("10.9.8.7")),
        catalogue_max_age_minutes,
        catalogue_refresh_seconds: 3600,
        zcash_rpc_mode: ZcashRpcMode::Unix,
        zcash_rpc_socket_path: String::from("/var/run/zcashd/zcashd.sock"),
        zcash_rpc_url: String::from("http://127.0.0.1:18232"),
        zcash_rpc_user: SecretString::from(String::from("rpc_user")),
        zcash_rpc_password: SecretString::from(String::from("rpc_password")),
        zcash_network: ZcashNetwork::Testnet,
        zcash_backend: ZcashBackend::Rpc,
        mock_zcash_auto_confirm: false,
        zingo_indexer_uri: String::from("https://testnet.zec.rocks"),
        zingo_wallet_dir: String::from("/tmp/zingo-wallet"),
        zingo_wallet_birthday: 0,
        zingo_init_timeout_seconds: 90,
        zingo_sync_retries: 3,
        zingo_sync_retry_delay_ms: 2000,
        required_confs_transparent: 2,
        required_confs_shielded: 3,
        order_expiry_minutes: 30,
        rate_lock_minutes: 15,
        sweep_threshold_zec: Decimal::new(5, 1),
        signing_service_url: String::from("http://10.0.0.2:8080"),
        signing_service_hmac_secret: SecretString::from(String::from("hmac_secret")),
        rate_source_timeout_ms: 3000,
        jwt_secret: SecretString::from(String::from(http::DEV_JWT_SECRET)),
        access_token_ttl_minutes: 15,
        refresh_token_ttl_hours: 24,
        brevo_api_key: None,
        brevo_sender_email: None,
        brevo_sender_name: None,
        app_base_url: String::from("http://localhost:3000"),
        frontend_url: None,
        cookie_domain: None,
        admin_secret: None,
    }
}

#[allow(dead_code)]
pub fn live_rate_cache() -> zutility_be::integrations::rates::SharedRateCache {
    use rust_decimal::Decimal;
    use zutility_be::integrations::rates::{CurrentRate, new_shared_rate_cache};

    new_shared_rate_cache(CurrentRate {
        zec_ngn: Decimal::new(1_500_000_000, 4),
        zec_usd: Decimal::new(1_000_000, 4),
        usd_ngn: Decimal::new(15_000_000, 4),
        usd_kes: Decimal::new(1_290_000, 4),
        usd_ghs: Decimal::new(120_000, 4),
        usd_zar: Decimal::new(180_000, 4),
        usd_egp: Decimal::new(300_000, 4),
        updated_at: chrono::Utc::now(),
    })
}

#[allow(dead_code)]
pub fn authed_app_with_config(pool: PgPool, config: &AppConfig, session: &Session) -> Router {
    let state = http::build_state(config, Some(live_rate_cache()), pool);
    http::build_router_from_state(state, false).layer(middleware::from_fn({
        let cookie = format!("access_token={}; csrf_token={}", session.jwt, session.csrf);
        let csrf = session.csrf.clone();
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
        }
    }))
}
