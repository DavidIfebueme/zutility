#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, Query, State, connect_info::MockConnectInfo},
    http::{Request, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::{Duration as ChronoDuration, Utc};
use rust_decimal::Decimal;
use secrecy::SecretString;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;
use zutility_be::{
    config::{AppConfig, AppEnv, ZcashBackend, ZcashNetwork, ZcashRpcMode},
    db::{self, CreateOrderInput},
    domain::money::Money,
    http,
    integrations::{
        dtone::{
            DtoneClient, DtoneSettings, EXTERNAL_ID_ALREADY_USED, PROVIDER_NAME, map_status_class,
        },
        utility_provider::{ProviderTxnStatus, UtilityProvider, UtilityPurchaseRequest},
    },
};

const CALLBACK_IP: &str = "10.9.8.7";
const CALLBACK_USER: &str = "dtone-callback";
const CALLBACK_PASS: &str = "dtone-callback-pass";
const ACCOUNT_NUMBER: &str = "0123456789";

#[derive(Clone, Default)]
struct MockDtone {
    requests: Arc<Mutex<Vec<(String, String, Value)>>>,
    transaction: Arc<Mutex<Value>>,
    create_failure: Arc<Mutex<Option<(u16, Value)>>>,
}

impl MockDtone {
    async fn record(&self, method: &str, uri: &Uri, body: Value) {
        let target = match uri.query() {
            Some(query) => format!("{}?{query}", uri.path()),
            None => uri.path().to_owned(),
        };
        self.requests
            .lock()
            .unwrap()
            .push((method.to_owned(), target, body));
    }

    async fn recorded(&self) -> Vec<(String, String, Value)> {
        self.requests.lock().unwrap().clone()
    }

    async fn set_transaction(&self, transaction: Value) {
        *self.transaction.lock().unwrap() = transaction;
    }

    async fn set_create_failure(&self, status: u16, payload: Value) {
        *self.create_failure.lock().unwrap() = Some((status, payload));
    }
}

fn transaction_payload(id: &str, external_id: &str, class_id: i64) -> Value {
    json!({
        "id": id,
        "external_id": external_id,
        "status": {
            "class": {"id": class_id, "message": "class"},
            "id": class_id + 1,
            "message": "status"
        },
        "pin": {"code": "pin-token-1"}
    })
}

async fn products_handler(
    State(state): State<MockDtone>,
    Query(query): Query<HashMap<String, String>>,
    uri: Uri,
) -> Response {
    state.record("GET", &uri, Value::Null).await;
    let page = query
        .get("page")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);

    let mut products = Vec::new();
    if page == 1 {
        for index in 0..100 {
            products.push(json!({"id": 40_000 + index, "name": format!("product-{index}")}));
        }
    } else if page == 2 {
        products.push(json!({"id": 45_082, "name": "Woyofal Senegal"}));
    }

    Json(Value::Array(products)).into_response()
}

async fn create_handler(State(state): State<MockDtone>, uri: Uri, body: String) -> Response {
    let payload = serde_json::from_str::<Value>(&body).unwrap_or(Value::Null);
    state.record("POST", &uri, payload).await;

    let failure = state.create_failure.lock().unwrap().clone();
    if let Some((status, payload)) = failure {
        let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST);
        return (status, Json(payload)).into_response();
    }

    let transaction = state.transaction.lock().unwrap().clone();
    (StatusCode::CREATED, Json(transaction)).into_response()
}

async fn transactions_handler(State(state): State<MockDtone>, uri: Uri) -> Response {
    state.record("GET", &uri, Value::Null).await;
    let transaction = state.transaction.lock().unwrap().clone();
    if transaction.is_null() {
        return Json(Value::Array(Vec::new())).into_response();
    }
    Json(Value::Array(vec![transaction])).into_response()
}

async fn transaction_handler(
    State(state): State<MockDtone>,
    Path(_id): Path<String>,
    uri: Uri,
) -> Response {
    state.record("GET", &uri, Value::Null).await;
    Json(state.transaction.lock().unwrap().clone()).into_response()
}

async fn balances_handler(State(state): State<MockDtone>, uri: Uri) -> Response {
    state.record("GET", &uri, Value::Null).await;
    Json(json!([{
        "id": 1,
        "unit_type": "CURRENCY",
        "unit": "NGN",
        "available": 250000,
        "holding": 0,
        "credit_limit": 0
    }]))
    .into_response()
}

async fn spawn_mock() -> (MockDtone, String) {
    let state = MockDtone::default();
    let app = Router::new()
        .route("/v1/products", get(products_handler))
        .route("/v1/async/transactions", post(create_handler))
        .route("/v1/transactions", get(transactions_handler))
        .route("/v1/transactions/{id}", get(transaction_handler))
        .route("/v1/balances", get(balances_handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock dtone");
    let address = listener.local_addr().expect("mock dtone address");

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (state, format!("http://{address}/v1"))
}

fn settings(base_url: &str, callback_ips: &[&str]) -> DtoneSettings {
    DtoneSettings {
        base_url: base_url.to_owned(),
        api_key: SecretString::from(String::from("dtone-key")),
        api_secret: SecretString::from(String::from("dtone-secret")),
        callback_url: String::from("https://api.zutility.test/api/v1/webhooks/dtone"),
        callback_user: SecretString::from(String::from(CALLBACK_USER)),
        callback_pass: SecretString::from(String::from(CALLBACK_PASS)),
        callback_ips: callback_ips
            .iter()
            .map(|value| value.parse::<IpAddr>().expect("callback ip"))
            .collect(),
    }
}

fn client(base_url: &str) -> DtoneClient {
    DtoneClient::new(settings(base_url, &[CALLBACK_IP]), Duration::from_secs(5))
        .expect("dtone client")
}

fn purchase_request(metadata: Value) -> UtilityPurchaseRequest {
    UtilityPurchaseRequest {
        order_id: Uuid::new_v4(),
        request_id: String::from("ext-1"),
        context: zutility_be::integrations::utility_provider::ProviderContext {
            utility_type: String::from("electricity"),
            service_id: String::from("woyofal-senegal"),
            billers_code: String::from(ACCOUNT_NUMBER),
            variation_code: None,
            meter_type: None,
            phone: None,
        },
        cost_ngn: Money::from_ngn_whole(1000).expect("naira"),
        settlement: Money::from_ngn_whole(1000).expect("naira"),
        product_id: String::from("45082"),
        external_id: String::from("ext-1"),
        inputs: BTreeMap::from([
            (String::from("account_number"), String::from(ACCOUNT_NUMBER)),
            (String::from("mobile_number"), String::from("221770000000")),
        ]),
        metadata,
        zec_amount: Decimal::new(1, 0),
    }
}

fn two_field_metadata() -> Value {
    json!({
        "amount_kind": "ranged",
        "required_fields": {
            "credit_party_identifier": ["account_number"],
            "beneficiary": ["mobile_number"]
        }
    })
}

fn basic_header(user: &str, password: &str) -> String {
    format!(
        "Basic {}",
        BASE64.encode(format!("{user}:{password}").as_bytes())
    )
}

#[tokio::test]
async fn every_status_class_maps_to_the_expected_status() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);

    let cases = [
        (10_001, ProviderTxnStatus::Pending),
        (20_001, ProviderTxnStatus::Pending),
        (50_001, ProviderTxnStatus::Pending),
        (70_001, ProviderTxnStatus::Delivered),
        (30_001, ProviderTxnStatus::Failed),
        (40_001, ProviderTxnStatus::Failed),
        (90_001, ProviderTxnStatus::Failed),
        (80_001, ProviderTxnStatus::Reversed),
    ];

    for (status_id, expected) in cases {
        let transaction_id = format!("txn-{status_id}");
        server
            .set_transaction(transaction_payload(&transaction_id, "ext-1", status_id))
            .await;

        let response = client
            .pay(&purchase_request(two_field_metadata()))
            .await
            .expect("dtone pay");

        assert_eq!(response.status, expected, "status id {status_id}");
        assert_eq!(response.provider_reference, transaction_id);
        assert_eq!(map_status_class(status_id / 10 * 10), expected);
    }
}

#[tokio::test]
async fn external_id_in_use_requeries_instead_of_failing() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);

    server
        .set_create_failure(
            400,
            json!({"errors": [{"code": EXTERNAL_ID_ALREADY_USED, "message": "already used"}]}),
        )
        .await;
    server
        .set_transaction(transaction_payload("txn-requeried", "ext-1", 70_000))
        .await;

    let response = client
        .pay(&purchase_request(two_field_metadata()))
        .await
        .expect("dtone pay resolves the conflict by requery");

    assert_eq!(response.status, ProviderTxnStatus::Delivered);
    assert_eq!(response.provider_reference, "txn-requeried");

    let recorded = server.recorded().await;
    assert!(recorded.iter().any(|(method, target, _)| {
        method == "GET" && target.starts_with("/v1/transactions?external_id=ext-1")
    }));
}

#[tokio::test]
async fn two_field_product_sends_both_identifier_objects() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);
    server
        .set_transaction(transaction_payload("txn-1", "ext-1", 20_000))
        .await;

    let response = client
        .pay(&purchase_request(two_field_metadata()))
        .await
        .expect("dtone pay");

    assert_eq!(response.status, ProviderTxnStatus::Pending);
    assert_eq!(response.token.as_deref(), Some("pin-token-1"));

    let recorded = server.recorded().await;
    let (_, target, body) = recorded
        .iter()
        .find(|(method, target, _)| method == "POST" && target == "/v1/async/transactions")
        .expect("create request");
    assert_eq!(target, "/v1/async/transactions");
    assert_eq!(body["external_id"], json!("ext-1"));
    assert_eq!(body["product_id"], json!(45_082));
    assert_eq!(body["auto_confirm"], json!(true));
    assert_eq!(
        body["callback_url"],
        json!("https://api.zutility.test/api/v1/webhooks/dtone")
    );
    assert_eq!(body["calculation_mode"], json!("DESTINATION_AMOUNT"));
    assert_eq!(body["destination"]["amount"], json!(1000));
    assert_eq!(
        body["credit_party_identifier"]["account_number"],
        json!(ACCOUNT_NUMBER)
    );
    assert_eq!(body["beneficiary"]["mobile_number"], json!("221770000000"));
}

#[tokio::test]
async fn fixed_product_omits_the_destination_block() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);
    server
        .set_transaction(transaction_payload("txn-1", "ext-1", 70_000))
        .await;

    client
        .pay(&purchase_request(json!({"amount_kind": "fixed"})))
        .await
        .expect("dtone pay");

    let recorded = server.recorded().await;
    let (_, _, body) = recorded
        .iter()
        .find(|(method, target, _)| method == "POST" && target == "/v1/async/transactions")
        .expect("create request");
    assert!(body.get("calculation_mode").is_none());
    assert!(body.get("destination").is_none());
}

#[tokio::test]
async fn missing_required_identifier_fails_before_the_http_call() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);
    server
        .set_transaction(transaction_payload("txn-1", "ext-1", 70_000))
        .await;

    let mut request = purchase_request(two_field_metadata());
    request.inputs.remove("mobile_number");

    let error = client.pay(&request).await.expect_err("missing identifier");
    assert!(!error.message.contains(ACCOUNT_NUMBER));
    assert!(error.message.contains("mobile_number"));

    let recorded = server.recorded().await;
    assert!(recorded.is_empty());
}

#[tokio::test]
async fn products_pages_until_a_short_page() {
    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);

    let products = client.products("SEN", 3).await.expect("dtone products");
    assert_eq!(products.len(), 101);
    assert_eq!(products[100].id, "45082");
    assert_eq!(products[100].name, "Woyofal Senegal");
    assert_eq!(products[100].country_iso3, "SEN");

    let recorded = server.recorded().await;
    let pages: Vec<&String> = recorded
        .iter()
        .filter(|(method, target, _)| method == "GET" && target.starts_with("/v1/products?"))
        .map(|(_, target, _)| target)
        .collect();
    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("page=1"));
    assert!(pages[1].contains("page=2"));
    assert!(pages[0].contains("country_iso_code=SEN"));
    assert!(pages[0].contains("service_id=3"));
}

#[tokio::test]
async fn balance_reads_the_funded_currency() {
    let (_server, base_url) = spawn_mock().await;
    let client = client(&base_url);

    let balances = client.balance().await.expect("dtone balance");
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].unit, "NGN");
    assert_eq!(balances[0].balance.available, Decimal::from(250_000));
}

#[test]
fn callback_basic_auth_accepts_only_the_configured_credentials() {
    let client = DtoneClient::new(
        settings("https://dtone.example/v1", &[CALLBACK_IP]),
        Duration::from_secs(5),
    )
    .expect("dtone client");

    let good = header_map("authorization", &basic_header(CALLBACK_USER, CALLBACK_PASS));
    assert!(client.verify_webhook(&good, b"{}"));

    let bad_password = header_map("authorization", &basic_header(CALLBACK_USER, "wrong"));
    assert!(!client.verify_webhook(&bad_password, b"{}"));

    let bad_user = header_map("authorization", &basic_header("wrong", CALLBACK_PASS));
    assert!(!client.verify_webhook(&bad_user, b"{}"));

    assert!(!client.verify_webhook(&header_map("x-other", "value"), b"{}"));
}

fn header_map(name: &'static str, value: &str) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::HeaderName::from_static(name),
        axum::http::HeaderValue::from_str(value).expect("header"),
    );
    headers
}

fn base_config(base_url: &str, callback_ips: &str) -> AppConfig {
    AppConfig {
        app_env: AppEnv::Dev,
        http_bind_addr: "127.0.0.1:3001".parse().expect("bind addr"),
        database_url: String::from("postgres://postgres:postgres@127.0.0.1:1/zutility"),
        order_token_hmac_secret: SecretString::from(String::from("order_secret")),
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
        inlomax_api_key: None,
        inlomax_base_url: None,
        inlomax_webhook_secret: None,
        dtone_api_key: Some(SecretString::from(String::from("dtone-key"))),
        dtone_api_secret: Some(SecretString::from(String::from("dtone-secret"))),
        dtone_base_url: Some(base_url.to_owned()),
        dtone_callback_url: Some(String::from(
            "https://api.zutility.test/api/v1/webhooks/dtone",
        )),
        dtone_callback_user: Some(SecretString::from(String::from(CALLBACK_USER))),
        dtone_callback_pass: Some(SecretString::from(String::from(CALLBACK_PASS))),
        dtone_callback_ips: Some(callback_ips.to_owned()),
        catalogue_max_age_minutes: 1440,
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
        required_confs_shielded: 2,
        order_expiry_minutes: 30,
        rate_lock_minutes: 15,
        sweep_threshold_zec: Decimal::new(5, 1),
        signing_service_url: String::from("http://10.0.0.2:8080"),
        signing_service_hmac_secret: SecretString::from(String::from("hmac_secret")),
        rate_source_timeout_ms: 3000,
        jwt_secret: SecretString::from(String::from("jwt_secret")),
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

fn callback_router(config: &AppConfig, pool: PgPool, peer: &str) -> Router {
    let peer = format!("{peer}:5000");
    let state = http::build_state(config, None, pool);
    http::build_router_from_state(state, false).layer(MockConnectInfo(
        peer.parse::<SocketAddr>().expect("peer address"),
    ))
}

async fn callback_request(
    router: Router,
    authorization: Option<&str>,
    extra_header: Option<(&'static str, &str)>,
    body: String,
) -> StatusCode {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks/dtone")
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(value) = authorization {
        builder = builder.header(header::AUTHORIZATION, value);
    }
    if let Some((name, value)) = extra_header {
        builder = builder.header(name, value);
    }

    router
        .oneshot(builder.body(axum::body::Body::from(body)).expect("request"))
        .await
        .expect("response")
        .status()
}

fn callback_body(order_id: &Uuid, transaction_id: &str, class_id: i64) -> String {
    transaction_payload(transaction_id, &order_id.to_string(), class_id).to_string()
}

#[tokio::test]
async fn callback_rejects_bad_credentials_and_forged_forwarded_addresses() {
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1:1/zutility")
        .expect("lazy pool");
    let mut config = base_config("https://dtone.example/v1", CALLBACK_IP);
    config.database_url = String::from("postgres://postgres:postgres@127.0.0.1:1/zutility");

    let order_id = Uuid::new_v4();
    let body = callback_body(&order_id, "txn-denied", 70_000);

    let wrong_password = callback_request(
        callback_router(&config, pool.clone(), CALLBACK_IP),
        Some(&basic_header(CALLBACK_USER, "wrong")),
        None,
        body.clone(),
    )
    .await;
    assert_eq!(wrong_password, StatusCode::UNAUTHORIZED);

    let no_credentials = callback_request(
        callback_router(&config, pool.clone(), CALLBACK_IP),
        None,
        None,
        body.clone(),
    )
    .await;
    assert_eq!(no_credentials, StatusCode::UNAUTHORIZED);

    let forged_forwarded_for = callback_request(
        callback_router(&config, pool.clone(), "203.0.113.9"),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        Some(("x-forwarded-for", CALLBACK_IP)),
        body.clone(),
    )
    .await;
    assert_eq!(forged_forwarded_for, StatusCode::UNAUTHORIZED);

    let public_peer_spoofing_real_ip = callback_request(
        callback_router(&config, pool.clone(), "203.0.113.9"),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        Some(("x-real-ip", CALLBACK_IP)),
        body.clone(),
    )
    .await;
    assert_eq!(public_peer_spoofing_real_ip, StatusCode::UNAUTHORIZED);

    let empty_allowlist = callback_request(
        callback_router(
            &base_config("https://dtone.example/v1", ""),
            pool.clone(),
            CALLBACK_IP,
        ),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        None,
        body,
    )
    .await;
    assert_eq!(empty_allowlist, StatusCode::UNAUTHORIZED);
}

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(pool)
}

async fn seed_dispatching_order(pool: &PgPool) -> Uuid {
    let prefix = format!("ztest-dtone-{}-", std::process::id());
    for index in 0..20 {
        sqlx::query(
            "INSERT INTO deposit_addresses (address, address_type, network, used)
             VALUES ($1, 'shielded', 'testnet', false)
             ON CONFLICT (address) DO NOTHING",
        )
        .bind(format!("{prefix}{index}"))
        .execute(pool)
        .await
        .expect("insert deposit address");
    }

    let rate_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM rate_snapshots ORDER BY fetched_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .expect("rate lookup");

    let rate_id = match rate_id {
        Some(rate_id) => rate_id,
        None => {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO rate_snapshots (id, zec_ngn, zec_usd, usd_ngn, sources_used, fetched_at)
                 VALUES ($1, 1000000, 50, 2000, $2, $3)",
            )
            .bind(id)
            .bind(vec![String::from("test")])
            .bind(Utc::now())
            .execute(pool)
            .await
            .expect("insert rate snapshot");
            id
        }
    };

    let mut tx = db::begin_tx(pool).await.expect("begin tx");
    let (order_id, _address) = db::insert_order_with_claimed_address(
        &mut tx,
        &CreateOrderInput {
            access_token_hash: String::from("token-hash"),
            utility_type: String::from("electricity"),
            utility_slug: String::from("woyofal-senegal"),
            service_ref: String::from(ACCOUNT_NUMBER),
            amount_ngn: 1000,
            country_iso2: String::from("NG"),
            product_id: None,
            settlement: zutility_be::domain::money::Money::from_ngn_whole(1000)
                .expect("naira settlement"),
            cost_amount: Decimal::from(1000),
            inputs: std::collections::BTreeMap::new(),
            fulfilment_kind: None,
            address_type: String::from("shielded"),
            zec_amount: Decimal::new(1, 0),
            zec_rate_id: rate_id,
            required_confs: 2,
            expires_at: Utc::now() + ChronoDuration::minutes(20),
            ip_hash: Some(String::from("hash")),
            metadata: json!({"source": "dtone-test"}),
            variation_code: None,
        },
        "testnet",
    )
    .await
    .expect("insert order");
    tx.commit().await.expect("commit order");

    sqlx::query(
        "UPDATE orders SET status = 'utility_dispatching', vtpass_request_id = $2 WHERE id = $1",
    )
    .bind(order_id)
    .bind(order_id.to_string())
    .execute(pool)
    .await
    .expect("mark dispatching");

    order_id
}

#[tokio::test]
async fn completed_then_reversed_records_two_events_and_applies_both() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let (server, base_url) = spawn_mock().await;

    let order_id = seed_dispatching_order(&pool).await;
    let external_id = order_id.to_string();

    let transaction_id = format!("txn-{}", Uuid::new_v4().simple());
    server
        .set_transaction(transaction_payload(&transaction_id, &external_id, 70_000))
        .await;

    let mut config = base_config(&base_url, CALLBACK_IP);
    config.database_url = std::env::var("TEST_DATABASE_URL").expect("test database url");

    let accepted = callback_request(
        callback_router(&config, pool.clone(), CALLBACK_IP),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        None,
        callback_body(&order_id, &transaction_id, 70_000),
    )
    .await;
    assert_eq!(accepted, StatusCode::OK);

    let status = sqlx::query_scalar::<_, String>("SELECT status FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("order status");
    assert_eq!(status, "completed");

    let token =
        sqlx::query_scalar::<_, Option<String>>("SELECT delivery_token FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .expect("delivery token");
    assert_eq!(token.as_deref(), Some("pin-token-1"));

    let replayed = callback_request(
        callback_router(&config, pool.clone(), CALLBACK_IP),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        None,
        callback_body(&order_id, &transaction_id, 70_000),
    )
    .await;
    assert_eq!(replayed, StatusCode::OK);

    server
        .set_transaction(transaction_payload(&transaction_id, &external_id, 80_000))
        .await;

    let reversed = callback_request(
        callback_router(&config, pool.clone(), CALLBACK_IP),
        Some(&basic_header(CALLBACK_USER, CALLBACK_PASS)),
        None,
        callback_body(&order_id, &transaction_id, 80_000),
    )
    .await;
    assert_eq!(reversed, StatusCode::OK);

    let status = sqlx::query_scalar::<_, String>("SELECT status FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("order status after reversal");
    assert_eq!(status, "reversed");

    let flag_reason =
        sqlx::query_scalar::<_, Option<String>>("SELECT flag_reason FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .expect("order flag reason after reversal");
    assert_eq!(flag_reason.as_deref(), Some("dtone_reversed"));

    let keys = sqlx::query_scalar::<_, String>(
        "SELECT event_key FROM webhook_events WHERE provider = $1 AND event_key LIKE $2 ORDER BY event_key",
    )
    .bind(PROVIDER_NAME)
    .bind(format!("{transaction_id}:%"))
    .fetch_all(&pool)
    .await
    .expect("webhook events");

    assert_eq!(
        keys,
        vec![
            format!("{transaction_id}:70000"),
            format!("{transaction_id}:80000")
        ]
    );

    let audit_events = sqlx::query_scalar::<_, String>(
        "SELECT event FROM audit_log WHERE order_id = $1 ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(&pool)
    .await
    .expect("audit events");
    assert_eq!(
        audit_events,
        vec![
            String::from("provider_completed"),
            String::from("provider_reversed")
        ]
    );
}

#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl CapturedLog {
    fn contents(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap_or_default()
    }
}

impl Write for CapturedLog {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = CapturedLog;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
async fn the_account_number_never_reaches_a_log_line() {
    let capture = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(capture.clone())
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::new("trace"))
        .finish();

    let (server, base_url) = spawn_mock().await;
    let client = client(&base_url);
    server
        .set_transaction(transaction_payload("txn-log-ok", "ext-1", 70_000))
        .await;

    let guard = tracing::subscriber::set_default(subscriber);
    tracing::info!("dtone log capture is active");

    let success = client.pay(&purchase_request(two_field_metadata())).await;
    assert!(success.is_ok());

    server
        .set_create_failure(
            400,
            json!({"errors": [{"code": 1_000_000, "message": format!("bad account {ACCOUNT_NUMBER}")}]}),
        )
        .await;

    let error = client
        .pay(&purchase_request(two_field_metadata()))
        .await
        .expect_err("provider rejects the request");

    server
        .set_create_failure(
            400,
            json!({"errors": [{"code": EXTERNAL_ID_ALREADY_USED, "message": "already used"}]}),
        )
        .await;

    let requeried = client
        .pay(&purchase_request(two_field_metadata()))
        .await
        .expect("requery path succeeds");

    drop(guard);

    assert_eq!(requeried.status, ProviderTxnStatus::Delivered);
    assert!(!error.message.contains(ACCOUNT_NUMBER));

    let logs = capture.contents();
    assert!(logs.contains("dtone log capture is active"));
    assert!(
        !logs.contains(ACCOUNT_NUMBER),
        "logs leaked the account number"
    );
    assert!(
        !logs.contains("221770000000"),
        "logs leaked the beneficiary"
    );
}
