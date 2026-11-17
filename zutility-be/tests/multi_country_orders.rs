#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    routing::{get, post},
};
use chrono::{Duration as ChronoDuration, Utc};
use rust_decimal::Decimal;
use secrecy::SecretString;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use zutility_be::{
    db,
    domain::money::Money,
    integrations::{
        dtone::{DtoneClient, DtoneSettings, PROVIDER_NAME},
        inlomax::InlomaxClient,
        provider_dispatcher::ProviderDispatcher,
        remita_collections::RemitaCollectionsClient,
        utility_provider::{
            ProviderContext, ProviderErrorKind, UtilityProvider, UtilityPurchaseRequest,
        },
        vtpass::VtpassClient,
    },
    runtime,
};

mod support;

const RRR: &str = "171523991694";

#[derive(Clone)]
struct NewProduct {
    id: Uuid,
    provider: &'static str,
    failover: Option<&'static str>,
    provider_product_id: &'static str,
    country: &'static str,
    utility_type: &'static str,
    slug: &'static str,
    amount_kind: &'static str,
    amount_min: Option<Decimal>,
    amount_max: Option<Decimal>,
    amount_increment: Option<Decimal>,
    currency: &'static str,
    wholesale_rate: Decimal,
    wholesale_fee: Decimal,
    retail_rate: Option<Decimal>,
    retail_fee: Option<Decimal>,
    required_fields: Value,
    enabled: bool,
    synced_hours_ago: i64,
}

impl NewProduct {
    fn ranged(id: Uuid, country: &'static str, currency: &'static str, slug: &'static str) -> Self {
        Self {
            id,
            provider: PROVIDER_NAME,
            failover: None,
            provider_product_id: "9001",
            country,
            utility_type: "electricity",
            slug,
            amount_kind: "ranged",
            amount_min: Some(Decimal::from(500)),
            amount_max: Some(Decimal::from(100_000)),
            amount_increment: Some(Decimal::from(500)),
            currency,
            wholesale_rate: Decimal::new(120, 2),
            wholesale_fee: Decimal::from(50),
            retail_rate: Some(Decimal::new(130, 2)),
            retail_fee: Some(Decimal::from(25)),
            required_fields: json!({}),
            enabled: true,
            synced_hours_ago: 0,
        }
    }

    fn nigerian(id: Uuid, slug: &'static str) -> Self {
        Self {
            provider: "vtpass",
            amount_min: Some(Decimal::from(50)),
            amount_max: Some(Decimal::from(50_000)),
            amount_increment: Some(Decimal::from(50)),
            currency: "NGN",
            wholesale_rate: Decimal::ONE,
            wholesale_fee: Decimal::ZERO,
            retail_rate: None,
            retail_fee: None,
            ..Self::ranged(id, "NG", "NGN", slug)
        }
    }
}

async fn pool() -> Option<PgPool> {
    support::pool().await
}

async fn create_session(pool: &PgPool) -> support::Session {
    support::session(pool).await.expect("session")
}

async fn enable_country(pool: &PgPool, iso2: &str, enabled: bool) {
    sqlx::query("UPDATE countries SET enabled = $2 WHERE iso2 = $1")
        .bind(iso2)
        .bind(enabled)
        .execute(pool)
        .await
        .expect("update country");
}

async fn clear_test_catalogue(pool: &PgPool) {
    sqlx::query(
        "DELETE FROM orders WHERE product_id IN (SELECT id FROM provider_products WHERE slug LIKE 'mc-%')",
    )
    .execute(pool)
    .await
    .expect("clear test orders");

    sqlx::query(
        "DELETE FROM provider_product_variations WHERE product_id IN (SELECT id FROM provider_products WHERE slug LIKE 'mc-%')",
    )
    .execute(pool)
    .await
    .expect("clear test variations");

    sqlx::query("DELETE FROM provider_products WHERE slug LIKE 'mc-%'")
        .execute(pool)
        .await
        .expect("clear test products");

    sqlx::query("UPDATE countries SET enabled = (iso2 = 'NG')")
        .execute(pool)
        .await
        .expect("reset enabled countries");
}

async fn insert_product(pool: &PgPool, product: &NewProduct) {
    let synced_at = Utc::now() - ChronoDuration::hours(product.synced_hours_ago);

    sqlx::query(
        "INSERT INTO provider_products (
            id, provider, failover_provider, provider_product_id, country_iso2, utility_type,
            utility_category, slug, name, descriptor, icon_key, service_ref_label,
            service_ref_placeholder, amount_kind, amount_min, amount_max, amount_increment,
            destination_currency, fulfilment_kind, wholesale_rate, wholesale_fee, retail_rate,
            retail_fee, required_fields, enabled, synced_at
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
            'payment', $19, $20, $21, $22, $23, $24, $25
         )",
    )
    .bind(product.id)
    .bind(product.provider)
    .bind(product.failover)
    .bind(product.provider_product_id)
    .bind(product.country)
    .bind(product.utility_type)
    .bind(product.utility_type)
    .bind(product.slug)
    .bind(product.slug)
    .bind(product.slug)
    .bind(product.utility_type)
    .bind("Account")
    .bind("Enter the account")
    .bind(product.amount_kind)
    .bind(product.amount_min)
    .bind(product.amount_max)
    .bind(product.amount_increment)
    .bind(product.currency)
    .bind(product.wholesale_rate)
    .bind(product.wholesale_fee)
    .bind(product.retail_rate)
    .bind(product.retail_fee)
    .bind(&product.required_fields)
    .bind(product.enabled)
    .bind(synced_at)
    .execute(pool)
    .await
    .expect("insert test product");
}

async fn insert_variation(
    pool: &PgPool,
    product_id: Uuid,
    variation_code: &str,
    provider_product_id: &str,
    amount: i64,
) {
    sqlx::query(
        "INSERT INTO provider_product_variations (
            id, product_id, provider_product_id, variation_code, name, amount, enabled
         ) VALUES ($1, $2, $3, $4, $5, $6, true)",
    )
    .bind(Uuid::new_v4())
    .bind(product_id)
    .bind(provider_product_id)
    .bind(variation_code)
    .bind(variation_code)
    .bind(Decimal::from(amount))
    .execute(pool)
    .await
    .expect("insert variation");
}

async fn post_json(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let parsed = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, parsed)
}

fn create_payload(slug: &str, utility_type: &str, service_ref: &str) -> Value {
    json!({
        "utility_type": utility_type,
        "utility_slug": slug,
        "service_ref": service_ref,
        "zec_address_type": "shielded",
    })
}

async fn order_id_from(body: &Value) -> Uuid {
    Uuid::parse_str(
        body.get("order_id")
            .and_then(Value::as_str)
            .expect("order id"),
    )
    .expect("order id uuid")
}

fn error_message(body: &Value) -> String {
    body.get("error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

async fn load_order(pool: &PgPool, order_id: Uuid) -> db::OrderRow {
    db::get_order_by_id(pool, order_id)
        .await
        .expect("order lookup")
        .expect("order exists")
}

#[derive(Clone, Default)]
struct MockDtone {
    bodies: Arc<Mutex<Vec<Value>>>,
}

async fn dtone_create_handler(
    State(state): State<MockDtone>,
    Json(body): Json<Value>,
) -> Json<Value> {
    state.bodies.lock().unwrap().push(body);
    Json(json!({
        "id": "dt-1",
        "external_id": "external",
        "status": {"class": {"id": 20000, "message": "pending"}},
        "pin": {"code": "pin-1"}
    }))
}

async fn spawn_dtone() -> (MockDtone, String) {
    let state = MockDtone::default();
    let app = Router::new()
        .route("/v1/async/transactions", post(dtone_create_handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (state, format!("http://{address}/v1"))
}

fn dtone_client(base_url: &str) -> DtoneClient {
    DtoneClient::new(
        DtoneSettings {
            base_url: base_url.to_owned(),
            api_key: SecretString::from(String::from("key")),
            api_secret: SecretString::from(String::from("secret")),
            callback_url: String::from("https://api.zutility.test/api/v1/webhooks/dtone"),
            callback_user: SecretString::from(String::from("user")),
            callback_pass: SecretString::from(String::from("pass")),
            callback_ips: Vec::new(),
        },
        Duration::from_secs(5),
    )
    .expect("dtone client")
}

#[derive(Clone, Default)]
struct CallCounter {
    calls: Arc<AtomicUsize>,
}

async fn counted(State(state): State<CallCounter>) -> StatusCode {
    state.calls.fetch_add(1, Ordering::SeqCst);
    StatusCode::BAD_GATEWAY
}

async fn spawn_counter() -> (CallCounter, String) {
    let counter = CallCounter::default();
    let app = Router::new().fallback(counted).with_state(counter.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind counter");
    let address = listener.local_addr().expect("counter address");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (counter, format!("http://{address}"))
}

async fn spawn_remita_lookup(amount_ngn: i64) -> String {
    let app = Router::new().route(
        "/biller/lookup/{rrr}",
        get(
            move |axum::extract::Path(rrr): axum::extract::Path<String>| async move {
                Json(json!({
                    "status": "00",
                    "message": "ok",
                    "data": {
                        "rrr": rrr,
                        "amount": amount_ngn.to_string(),
                        "status": "pending",
                        "billerName": "State University",
                        "name": "Ada Obi"
                    }
                }))
            },
        ),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind remita");
    let address = listener.local_addr().expect("remita address");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    format!("http://{address}")
}

#[tokio::test]
async fn a_disabled_product_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let mut product = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000001),
        "mc-ng-off",
    );
    product.enabled = false;
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-ng-off", "airtime", "08000000001");
    payload["amount_ngn"] = json!(500);

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(error_message(&body), "product_unavailable");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_disabled_country_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000002),
        "ZM",
        "ZMW",
        "mc-zm-off",
    );
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-zm-off", "electricity", "0123456789");
    payload["country_iso2"] = json!("ZM");
    payload["settlement"] = json!({"currency": "ZMW", "amount": "1000"});

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(error_message(&body), "product_unavailable");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_settlement_outside_the_bounds_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "SN", true).await;

    let product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000003),
        "SN",
        "XOF",
        "mc-sn-biller",
    );
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-sn-biller", "electricity", "12345678");
    payload["country_iso2"] = json!("SN");
    payload["settlement"] = json!({"currency": "XOF", "amount": "100"});

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("bounds"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_settlement_off_the_increment_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "SN", true).await;

    let product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000004),
        "SN",
        "XOF",
        "mc-sn-biller",
    );
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-sn-biller", "electricity", "12345678");
    payload["country_iso2"] = json!("SN");
    payload["settlement"] = json!({"currency": "XOF", "amount": "1250"});

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("increment"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_settlement_in_the_wrong_currency_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "SN", true).await;

    let product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000005),
        "SN",
        "XOF",
        "mc-sn-biller",
    );
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-sn-biller", "electricity", "12345678");
    payload["country_iso2"] = json!("SN");
    payload["settlement"] = json!({"currency": "NGN", "amount": "1000"});

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("currency"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_variation_amount_mismatch_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let mut product = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000006),
        "mc-ng-data",
    );
    product.utility_type = "data";
    product.slug = "mc-ng-data";
    product.amount_kind = "variation";
    product.amount_min = None;
    product.amount_max = None;
    product.amount_increment = None;
    insert_product(&pool, &product).await;
    insert_variation(&pool, product.id, "tier-1", "tier-1", 1500).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-ng-data", "data", "08000000001");
    payload["amount_ngn"] = json!(1000);
    payload["variation_code"] = json!("tier-1");

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("variation"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_gift_for_a_disabled_product_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let mut product = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000007),
        "mc-ng-off",
    );
    product.utility_type = "data";
    product.slug = "mc-ng-off";
    product.enabled = false;
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let (status, body) = post_json(
        &app,
        "/api/v1/gifts",
        json!({
            "utility_type": "data",
            "utility_slug": "mc-ng-off",
            "amount_ngn": 1000,
            "zec_address_type": "shielded",
            "claim_window_hours": 24
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(error_message(&body), "product_unavailable");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_gift_for_a_non_naira_product_is_rejected() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let mut product = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000008),
        "mc-ng-usd",
    );
    product.utility_type = "data";
    product.slug = "mc-ng-usd";
    product.currency = "USD";
    product.retail_rate = Some(Decimal::new(1500, 0));
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let (status, body) = post_json(
        &app,
        "/api/v1/gifts",
        json!({
            "utility_type": "data",
            "utility_slug": "mc-ng-usd",
            "amount_ngn": 1000,
            "zec_address_type": "shielded",
            "claim_window_hours": 24
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("naira"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_variation_gift_must_match_its_tier() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let mut product = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000009),
        "mc-ng-data",
    );
    product.utility_type = "data";
    product.slug = "mc-ng-data";
    product.amount_kind = "variation";
    product.amount_min = None;
    product.amount_max = None;
    product.amount_increment = None;
    insert_product(&pool, &product).await;
    insert_variation(&pool, product.id, "tier-1", "tier-1", 1500).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let (status, body) = post_json(
        &app,
        "/api/v1/gifts",
        json!({
            "utility_type": "data",
            "utility_slug": "mc-ng-data",
            "amount_ngn": 500,
            "variation_code": "tier-1",
            "zec_address_type": "shielded",
            "claim_window_hours": 24
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(error_message(&body).contains("variation"), "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_school_fee_order_ignores_the_client_amount() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let remita_url = spawn_remita_lookup(250_000).await;
    let mut config = support::app_config(None, 1440);
    config.remita_collections_base_url = Some(remita_url);
    config.remita_collections_secret_key = Some(SecretString::from(String::from("secret")));

    let session = create_session(&pool).await;
    let app = support::authed_app_with_config(pool.clone(), &config, &session);

    let mut payload = create_payload("school-fees", "school_fees", RRR);
    payload["amount_ngn"] = json!(1);

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let order = load_order(&pool, order_id_from(&body).await).await;
    assert_eq!(order.amount_ngn, 250_000);
    assert_eq!(
        order.settlement.expect("settlement").amount(),
        Decimal::from(250_000)
    );
    assert_eq!(order.service_ref, RRR);

    clear_test_catalogue(&pool).await;
    sqlx::query("DELETE FROM orders WHERE utility_type = 'school_fees' AND service_ref = $1")
        .bind(RRR)
        .execute(&pool)
        .await
        .expect("clear school fee order");
}

#[tokio::test]
async fn a_stale_dtone_product_is_rejected_while_a_stale_nigerian_row_still_orders() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "SN", true).await;

    let mut senegalese = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000010),
        "SN",
        "XOF",
        "mc-sn-stale",
    );
    senegalese.synced_hours_ago = 100;
    insert_product(&pool, &senegalese).await;

    let mut nigerian = NewProduct::nigerian(
        Uuid::from_u128(0x4d430000000000000000000000000011),
        "mc-ng-stale",
    );
    nigerian.synced_hours_ago = 100;
    nigerian.utility_type = "data";
    nigerian.slug = "mc-ng-stale";
    nigerian.amount_kind = "variation";
    nigerian.amount_min = None;
    nigerian.amount_max = None;
    nigerian.amount_increment = None;
    insert_product(&pool, &nigerian).await;
    insert_variation(&pool, nigerian.id, "tier-1", "tier-1", 1500).await;

    let session = create_session(&pool).await;
    let app =
        support::authed_app_with_config(pool.clone(), &support::app_config(None, 1440), &session);

    let mut senegal = create_payload("mc-sn-stale", "electricity", "12345678");
    senegal["country_iso2"] = json!("SN");
    senegal["settlement"] = json!({"currency": "XOF", "amount": "1000"});
    let (status, body) = post_json(&app, "/api/v1/orders/create", senegal).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");

    let mut nigeria = create_payload("mc-ng-stale", "data", "08000000002");
    nigeria["amount_ngn"] = json!(1500);
    nigeria["variation_code"] = json!("tier-1");
    let (status, body) = post_json(&app, "/api/v1/orders/create", nigeria).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_senegal_order_persists_the_xof_settlement() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "SN", true).await;

    let product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000012),
        "SN",
        "XOF",
        "mc-sn-biller",
    );
    insert_product(&pool, &product).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-sn-biller", "electricity", "123 456 78");
    payload["country_iso2"] = json!("SN");
    payload["settlement"] = json!({"currency": "XOF", "amount": "1000"});

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let order = load_order(&pool, order_id_from(&body).await).await;
    let settlement = order.settlement.expect("settlement");

    assert_eq!(order.country_iso2, "SN");
    assert_eq!(settlement.currency().iso(), "XOF");
    assert_eq!(settlement.amount(), Decimal::from(1000));
    assert_eq!(order.amount_ngn, 1325);
    assert_eq!(order.cost.expect("cost").amount(), Decimal::from(1250));
    assert_eq!(order.service_ref, "12345678");
    assert_eq!(order.product_id, Some(product.id));

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_legacy_request_without_a_country_or_product_id_still_orders() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mtn", "airtime", "08000000003");
    payload["amount_ngn"] = json!(5000);

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let order = load_order(&pool, order_id_from(&body).await).await;
    let settlement = order.settlement.expect("settlement");

    assert_eq!(order.country_iso2, "NG");
    assert_eq!(order.utility_type, "airtime");
    assert_eq!(order.amount_ngn, 5000);
    assert_eq!(settlement.amount(), Decimal::from(5000));
    assert_eq!(settlement.currency().iso(), "NGN");
    assert!(order.product_id.is_some());

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn two_field_inputs_survive_create_and_dispatch() {
    let Some(pool) = pool().await else {
        return;
    };
    clear_test_catalogue(&pool).await;
    enable_country(&pool, "ZM", true).await;

    let mut product = NewProduct::ranged(
        Uuid::from_u128(0x4d430000000000000000000000000013),
        "ZM",
        "ZMW",
        "mc-zm-tv",
    );
    product.utility_type = "television";
    product.amount_kind = "variation";
    product.amount_min = None;
    product.amount_max = None;
    product.amount_increment = None;
    product.provider_product_id = "9101";
    product.required_fields = json!({
        "credit_party_identifier": ["account_number"],
        "beneficiary": ["mobile_number"]
    });
    insert_product(&pool, &product).await;
    insert_variation(&pool, product.id, "tier-1", "9102", 50).await;

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let mut payload = create_payload("mc-zm-tv", "television", "DSTV-1234");
    payload["product_id"] = json!(product.id);
    payload["variation_code"] = json!("tier-1");
    payload["settlement"] = json!({"currency": "ZMW", "amount": "50"});
    payload["inputs"] = json!({
        "account_number": "99887766",
        "mobile_number": "0977123456"
    });

    let (status, body) = post_json(&app, "/api/v1/orders/create", payload).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let order = load_order(&pool, order_id_from(&body).await).await;
    assert_eq!(order.inputs.len(), 2);
    assert_eq!(
        order.inputs.get("account_number").map(String::as_str),
        Some("99887766")
    );

    let stored_product = db::find_product_by_id(&pool, product.id)
        .await
        .expect("product lookup")
        .expect("product exists");
    let variation = db::find_product_variation(&pool, product.id, "tier-1")
        .await
        .expect("variation lookup");

    let request = runtime::purchase_request_for(
        &order,
        Some(&stored_product),
        variation.as_ref(),
        &order.id.to_string(),
    )
    .expect("purchase request");

    assert_eq!(request.product_id, "9102");
    assert_eq!(request.inputs.len(), 2);
    assert_eq!(
        request.metadata.get("amount_kind").and_then(Value::as_str),
        Some("variation")
    );

    let (mock, base_url) = spawn_dtone().await;
    let client = dtone_client(&base_url);
    client.pay(&request).await.expect("dtone create");

    let bodies = mock.bodies.lock().unwrap().clone();
    let sent = bodies.first().expect("captured body");
    assert_eq!(
        sent.pointer("/credit_party_identifier/account_number")
            .and_then(Value::as_str),
        Some("99887766")
    );
    assert_eq!(
        sent.pointer("/beneficiary/mobile_number")
            .and_then(Value::as_str),
        Some("0977123456")
    );
    assert_eq!(sent.get("product_id").and_then(Value::as_i64), Some(9102));

    clear_test_catalogue(&pool).await;
}

#[tokio::test]
async fn a_dtone_outage_never_calls_remita() {
    let (counter, counter_url) = spawn_counter().await;

    let inlomax = InlomaxClient::new(
        counter_url.clone(),
        SecretString::from(String::from("key")),
        SecretString::from(String::from("secret")),
        Duration::from_secs(5),
    )
    .expect("inlomax client");

    let remita_collections = RemitaCollectionsClient::new(
        counter_url.clone(),
        SecretString::from(String::from("secret")),
        Duration::from_secs(5),
    )
    .expect("remita collections client");

    let vtpass = VtpassClient::new(
        counter_url,
        SecretString::from(String::from("key")),
        SecretString::from(String::from("secret")),
        SecretString::from(String::from("hmac")),
        Duration::from_secs(5),
    )
    .expect("vtpass client");

    let dtone = dtone_client("http://127.0.0.1:1/v1");
    let dispatcher = ProviderDispatcher::new(
        vtpass,
        None,
        Some(remita_collections),
        Some(inlomax),
        Some(dtone),
    );

    let request = UtilityPurchaseRequest {
        order_id: Uuid::new_v4(),
        request_id: String::from("req-1"),
        context: ProviderContext {
            utility_type: String::from("electricity"),
            service_id: String::from("mc-sn-biller"),
            billers_code: String::from("12345678"),
            variation_code: None,
            meter_type: None,
            phone: None,
        },
        cost_ngn: Money::from_ngn_whole(1250).expect("cost"),
        settlement: Money::new(
            zutility_be::domain::money::CurrencyCode::Xof,
            Decimal::from(1000),
        )
        .expect("settlement"),
        product_id: String::from("9001"),
        external_id: String::from("external-1"),
        inputs: BTreeMap::new(),
        metadata: json!({"amount_kind": "ranged"}),
        zec_amount: Decimal::ONE,
    };

    let error = dispatcher
        .pay_for_product("dtone", None, &request)
        .await
        .expect_err("dtone outage");

    assert!(
        matches!(
            error.kind,
            ProviderErrorKind::Outage | ProviderErrorKind::Transient | ProviderErrorKind::Permanent
        ),
        "{error}"
    );
    assert_eq!(counter.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn completed_to_reversed_and_failed_to_completed_use_the_database_functions() {
    let Some(pool) = pool().await else {
        return;
    };

    let first = seed_order(&pool, "mc-reverse", "completed").await;
    assert!(
        db::mark_order_reversed(&pool, first, "80000", None)
            .await
            .expect("reverse")
    );
    let order = load_order(&pool, first).await;
    assert_eq!(order.status, "reversed");
    assert_eq!(order.flag_reason.as_deref(), Some("dtone_reversed"));
    assert_eq!(order.provider_status.as_deref(), Some("80000"));

    let second = seed_order(&pool, "mc-retry", "failed").await;
    assert!(
        db::complete_order(&pool, second, Some("pin"), Some("reference"))
            .await
            .expect("complete")
    );
    let order = load_order(&pool, second).await;
    assert_eq!(order.status, "completed");
    assert_eq!(order.delivery_token.as_deref(), Some("pin"));

    sqlx::query("DELETE FROM orders WHERE utility_slug LIKE 'mc-%'")
        .execute(&pool)
        .await
        .expect("clear orders");
}

#[tokio::test]
async fn a_reversal_is_found_by_provider_reference_after_completion() {
    let Some(pool) = pool().await else {
        return;
    };

    let order_id = seed_order(&pool, "mc-reference", "completed").await;
    let reference = format!("txn-{}", Uuid::new_v4().simple());

    sqlx::query("UPDATE orders SET provider_transaction_id = $2 WHERE id = $1")
        .bind(order_id)
        .bind(&reference)
        .execute(&pool)
        .await
        .expect("store provider transaction id");

    let found = db::find_order_by_provider_reference(&pool, &reference)
        .await
        .expect("find by reference")
        .expect("completed order is found");
    assert_eq!(found.id, order_id);
    assert_eq!(found.status, "completed");

    assert!(
        db::mark_order_reversed(&pool, order_id, "80000", None)
            .await
            .expect("reverse")
    );

    let found = db::find_order_by_provider_reference(&pool, &reference)
        .await
        .expect("find by reference")
        .expect("reversed order is found");
    assert_eq!(found.status, "reversed");

    sqlx::query("DELETE FROM orders WHERE id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .expect("clear order");
}

#[tokio::test]
async fn every_utility_type_and_status_is_accepted_by_the_schema() {
    let Some(pool) = pool().await else {
        return;
    };

    for utility_type in [
        "airtime",
        "data",
        "dstv",
        "gotv",
        "startimes",
        "showmax",
        "electricity",
        "school_fees",
        "waec",
        "jamb",
        "television",
        "internet",
        "landline",
        "voip",
    ] {
        insert_raw_order(&pool, utility_type, "awaiting_payment").await;
    }

    for status in [
        "awaiting_payment",
        "payment_detected",
        "payment_confirmed",
        "awaiting_claim",
        "utility_dispatching",
        "completed",
        "expired",
        "failed",
        "flagged_for_review",
        "cancelled",
        "reversed",
    ] {
        insert_raw_order(&pool, "airtime", status).await;
    }

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM orders WHERE utility_slug LIKE 'mc-schema-%'",
    )
    .fetch_one(&pool)
    .await
    .expect("count schema rows");
    assert_eq!(count, 25);

    sqlx::query("DELETE FROM orders WHERE utility_slug LIKE 'mc-schema-%'")
        .execute(&pool)
        .await
        .expect("clear schema rows");
}

async fn insert_raw_order(pool: &PgPool, utility_type: &str, status: &str) -> Uuid {
    let order_id = Uuid::new_v4();
    let address = format!("mc-schema-{order_id}");
    let rate_id = db::persist_rate_snapshot(
        pool,
        &db::PersistRateSnapshotInput {
            zec_ngn: Decimal::from(1_000_000),
            zec_usd: Decimal::from(50),
            usd_ngn: Decimal::from(2000),
            usd_kes: Decimal::from(100),
            usd_ghs: Decimal::from(10),
            usd_zar: Decimal::from(18),
            usd_egp: Decimal::from(50),
            coingecko_zec_ngn: None,
            binance_zec_usd: None,
            kraken_zec_usd: None,
            coinbase_zec_usd: None,
            sources_used: vec![String::from("test")],
            fetched_at: Utc::now(),
        },
    )
    .await
    .expect("rate snapshot");

    sqlx::query(
        "INSERT INTO deposit_addresses (address, address_type, network, used)
         VALUES ($1, 'shielded', 'testnet', true)",
    )
    .bind(&address)
    .execute(pool)
    .await
    .expect("deposit address");

    sqlx::query(
        "INSERT INTO orders (
            id, status, access_token_hash, utility_type, utility_slug, service_ref, amount_ngn,
            deposit_address, address_type, zec_amount, zec_rate_id, required_confs, expires_at
         ) VALUES ($1, $2, 'hash', $3, $4, 'ref', 1000, $5, 'shielded', 1, $6, 2, now() + interval '20 minutes')",
    )
    .bind(order_id)
    .bind(status)
    .bind(utility_type)
    .bind(format!("mc-schema-{utility_type}-{status}"))
    .bind(&address)
    .bind(rate_id)
    .execute(pool)
    .await
    .expect("insert raw order");

    order_id
}

async fn seed_order(pool: &PgPool, slug: &str, status: &str) -> Uuid {
    let order_id = Uuid::new_v4();
    let address = format!("mc-order-{order_id}");
    let rate_id = db::persist_rate_snapshot(
        pool,
        &db::PersistRateSnapshotInput {
            zec_ngn: Decimal::from(1_000_000),
            zec_usd: Decimal::from(50),
            usd_ngn: Decimal::from(2000),
            usd_kes: Decimal::from(100),
            usd_ghs: Decimal::from(10),
            usd_zar: Decimal::from(18),
            usd_egp: Decimal::from(50),
            coingecko_zec_ngn: None,
            binance_zec_usd: None,
            kraken_zec_usd: None,
            coinbase_zec_usd: None,
            sources_used: vec![String::from("test")],
            fetched_at: Utc::now(),
        },
    )
    .await
    .expect("rate snapshot");

    sqlx::query(
        "INSERT INTO deposit_addresses (address, address_type, network, used)
         VALUES ($1, 'shielded', 'testnet', true)",
    )
    .bind(&address)
    .execute(pool)
    .await
    .expect("deposit address");

    sqlx::query(
        "INSERT INTO orders (
            id, status, access_token_hash, utility_type, utility_slug, service_ref, amount_ngn,
            country_iso2, settlement_currency, settlement_amount, cost_currency, cost_amount,
            deposit_address, address_type, zec_amount, zec_rate_id, required_confs, expires_at
         ) VALUES ($1, $2, 'hash', 'electricity', $3, 'ref', 1000, 'SN', 'XOF', 1000, 'NGN', 1250,
                   $4, 'shielded', 1, $5, 2, now() + interval '20 minutes')",
    )
    .bind(order_id)
    .bind(status)
    .bind(slug)
    .bind(&address)
    .bind(rate_id)
    .execute(pool)
    .await
    .expect("insert order");

    order_id
}

#[tokio::test]
async fn a_flagged_order_is_active_for_every_utility() {
    let Some(pool) = pool().await else {
        return;
    };

    let order_id = seed_order(&pool, "mc-flagged", "flagged_for_review").await;
    sqlx::query("UPDATE orders SET flag_reason = 'dtone_dispatch_timeout' WHERE id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .expect("set flag reason");

    let active = db::list_active_orders(&pool).await.expect("active orders");
    assert!(active.iter().any(|order| order.id == order_id));

    sqlx::query("DELETE FROM orders WHERE id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .expect("clear order");
}

#[tokio::test]
async fn a_reversed_order_reports_its_status_through_the_api() {
    let Some(pool) = pool().await else {
        return;
    };

    let order_id = seed_order(&pool, "mc-status", "completed").await;
    let token = String::from("order-token");
    let hash = zutility_be::http::auth::hash_order_token(
        &SecretString::from(String::from("dev_order_token_secret")),
        &token,
    )
    .expect("order token hash");
    sqlx::query("UPDATE orders SET access_token_hash = $2 WHERE id = $1")
        .bind(order_id)
        .bind(&hash)
        .execute(&pool)
        .await
        .expect("store token hash");

    assert!(
        db::mark_order_reversed(&pool, order_id, "80000", Some("dtone_reversed"))
            .await
            .expect("reverse")
    );

    let session = create_session(&pool).await;
    let app = support::authed_app(pool.clone(), &session);
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orders/{order_id}?token={token}"))
        .body(Body::empty())
        .expect("build request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let body: Value = serde_json::from_slice(&bytes).expect("json body");

    assert_eq!(body.get("status").and_then(Value::as_str), Some("reversed"));
    assert_eq!(
        body.get("failure_code").and_then(Value::as_str),
        Some("dtone_reversed")
    );

    sqlx::query("DELETE FROM orders WHERE id = $1")
        .bind(order_id)
        .execute(&pool)
        .await
        .expect("clear order");
}
