#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, Uri},
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use secrecy::SecretString;
use serde_json::{Value, json};
use sha2::Sha256;
use tokio::sync::Mutex;
use uuid::Uuid;
use zutility_be::{
    domain::money::Money,
    integrations::{
        inlomax::InlomaxClient,
        remita::RemitaClient,
        remita_collections::RemitaCollectionsClient,
        utility_provider::{ProviderContext, UtilityProvider, UtilityPurchaseRequest},
        vtpass::VtpassClient,
    },
};

const ORDER_ID: &str = "00000000-0000-4000-8000-000000000001";
const REMITA_HASH: &str = "a35f145fe1d33f81169db68541eae769702d2fee431b65ab9125529b48a0ec7648f115fd6d1956ca274bf39b39b32638054acead7a21777011cd1e30cced67bb";

#[derive(Clone, Default)]
struct MockServer {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
}

impl MockServer {
    async fn recorded(&self) -> Vec<(String, Value)> {
        self.requests.lock().await.clone()
    }
}

async fn inlomax_handler(
    State(state): State<MockServer>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((uri.path().to_owned(), body));

    Json(json!({"status": "success", "data": {"reference": "inl-ref", "token": "inl-token"}}))
}

async fn vtpass_handler(
    State(state): State<MockServer>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((uri.path().to_owned(), body));

    Json(json!({
        "code": "000",
        "response_description": "TRANSACTION SUCCESSFUL",
        "request_id": "req-1",
        "content": {"transactionId": "vp-tx"},
        "token": "vp-token"
    }))
}

async fn remita_handler(
    State(state): State<MockServer>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((uri.path().to_owned(), body));

    Json(json!({"status_code": "021", "status": "00", "rrr": "rrr-1"}))
}

async fn remita_lookup_handler(
    State(state): State<MockServer>,
    Path(rrr): Path<String>,
) -> Json<Value> {
    let path = format!("/remita-collections/biller/lookup/{rrr}");
    state
        .requests
        .lock()
        .await
        .push((path, serde_json::Value::Null));

    Json(json!({
        "status": "00",
        "message": "ok",
        "data": {
            "rrr": rrr,
            "amount": 1000,
            "fee": 0,
            "status": "pending",
            "billerName": "NSITF",
            "name": "Test User"
        }
    }))
}

async fn remita_process_handler(
    State(state): State<MockServer>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((uri.path().to_owned(), body));

    Json(json!({
        "status": "00",
        "message": "ok",
        "data": {"transactionStatus": "success"}
    }))
}

async fn spawn_server() -> (MockServer, String) {
    let state = MockServer::default();
    let app = Router::new()
        .route("/inlomax/{*rest}", post(inlomax_handler))
        .route("/vtpass/{*rest}", post(vtpass_handler))
        .route("/remita/{*rest}", post(remita_handler))
        .route(
            "/remita-collections/biller/lookup/{rrr}",
            get(remita_lookup_handler),
        )
        .route(
            "/remita-collections/biller/process",
            post(remita_process_handler),
        )
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock provider");
    let addr = listener.local_addr().expect("mock provider address");

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (state, format!("http://{addr}"))
}

fn naira(amount: i64) -> Money {
    Money::from_ngn_whole(amount).expect("naira amount")
}

fn context(utility_type: &str, service_id: &str, billers_code: &str) -> ProviderContext {
    ProviderContext {
        utility_type: utility_type.to_owned(),
        service_id: service_id.to_owned(),
        billers_code: billers_code.to_owned(),
        variation_code: Some(String::from("var-1")),
        meter_type: Some(String::from("postpaid")),
        phone: Some(String::from("08000000000")),
    }
}

fn purchase_request(context: ProviderContext) -> UtilityPurchaseRequest {
    UtilityPurchaseRequest {
        order_id: Uuid::parse_str(ORDER_ID).expect("order id"),
        request_id: String::from("req-1"),
        context,
        cost_ngn: naira(1000),
        settlement: naira(1000),
        product_id: String::from("product-1"),
        external_id: String::from("ext-1"),
        inputs: BTreeMap::from([(String::from("account_number"), String::from("0123456789"))]),
        metadata: json!({}),
        zec_amount: Decimal::new(1, 0),
    }
}

fn header_map(name: &'static str, value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(name, HeaderValue::from_str(value).expect("header value"));
    headers
}

fn hmac_hex(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

#[tokio::test]
async fn vtpass_receives_whole_naira_and_context() {
    let (server, base_url) = spawn_server().await;
    let client = VtpassClient::new(
        format!("{base_url}/vtpass"),
        SecretString::from(String::from("api")),
        SecretString::from(String::from("secret")),
        SecretString::from(String::from("webhook")),
        Duration::from_secs(5),
    )
    .expect("vtpass client");

    client
        .pay(&purchase_request(context("airtime", "mtn", "08000000000")))
        .await
        .expect("vtpass pay");

    let recorded = server.recorded().await;
    let (path, body) = recorded.last().expect("vtpass request");
    assert_eq!(path, "/vtpass/pay");
    assert_eq!(body.get("amount").and_then(Value::as_i64), Some(1000));
    assert_eq!(body.get("serviceID").and_then(Value::as_str), Some("mtn"));
    assert_eq!(
        body.get("billersCode").and_then(Value::as_str),
        Some("08000000000")
    );
    assert_eq!(
        body.get("variation_code").and_then(Value::as_str),
        Some("var-1")
    );
    assert_eq!(
        body.get("phone").and_then(Value::as_str),
        Some("08000000000")
    );
}

#[test]
fn remita_hash_matches_recorded_value() {
    let client = RemitaClient::new(
        String::from("https://remita.example"),
        String::from("ZUTILITY"),
        SecretString::from(String::from("remita-api-key")),
        String::from("4430731"),
        SecretString::from(String::from("remita-webhook")),
        Duration::from_secs(5),
    )
    .expect("remita client");

    assert_eq!(client.compute_hash(ORDER_ID, 1000), REMITA_HASH);
}

#[tokio::test]
async fn remita_pay_sends_recorded_hash_and_context() {
    let (server, base_url) = spawn_server().await;
    let client = RemitaClient::new(
        format!("{base_url}/remita"),
        String::from("ZUTILITY"),
        SecretString::from(String::from("remita-api-key")),
        String::from("4430731"),
        SecretString::from(String::from("remita-webhook")),
        Duration::from_secs(5),
    )
    .expect("remita client");

    client
        .pay(&purchase_request(context(
            "school_fees",
            "school-fees",
            "141523991664",
        )))
        .await
        .expect("remita pay");

    let recorded = server.recorded().await;
    let (path, body) = recorded.last().expect("remita request");
    assert_eq!(path, "/remita/billpayment/pay");
    assert_eq!(body.get("amount").and_then(Value::as_i64), Some(1000));
    assert_eq!(body.get("hash").and_then(Value::as_str), Some(REMITA_HASH));
    assert!(body.get("payerPhone").is_none());
    assert_eq!(
        body.get("description").and_then(Value::as_str),
        Some("Zutility: school-fees for 141523991664")
    );
}

#[tokio::test]
async fn inlomax_chooses_endpoint_from_utility_type() {
    let (server, base_url) = spawn_server().await;
    let client = InlomaxClient::new(
        format!("{base_url}/inlomax"),
        SecretString::from(String::from("inlomax-api")),
        SecretString::from(String::from("inlomax-webhook")),
        Duration::from_secs(5),
    )
    .expect("inlomax client");

    client
        .pay(&purchase_request(context(
            "electricity",
            "ikeja-electric",
            "04040404040",
        )))
        .await
        .expect("inlomax pay");

    let recorded = server.recorded().await;
    let (path, body) = recorded.last().expect("inlomax request");
    assert_eq!(path, "/inlomax/payelectric");
    assert_eq!(body.get("amount").and_then(Value::as_i64), Some(1000));
    assert_eq!(body.get("serviceID").and_then(Value::as_str), Some("1"));
    assert_eq!(
        body.get("meterNum").and_then(Value::as_str),
        Some("04040404040")
    );
    assert_eq!(body.get("meterType").and_then(Value::as_i64), Some(2));
}

#[tokio::test]
async fn remita_collections_receives_whole_naira_and_context() {
    let (server, base_url) = spawn_server().await;
    let client = RemitaCollectionsClient::new(
        format!("{base_url}/remita-collections"),
        SecretString::from(String::from("remita-collections-key")),
        Duration::from_secs(5),
    )
    .expect("remita collections client");

    client
        .pay(&purchase_request(context(
            "school_fees",
            "school-fees",
            "141523991664",
        )))
        .await
        .expect("remita collections pay");

    let recorded = server.recorded().await;
    assert!(
        recorded
            .iter()
            .any(|(path, _)| path == "/remita-collections/biller/lookup/141523991664")
    );

    let (path, body) = recorded.last().expect("remita process request");
    assert_eq!(path, "/remita-collections/biller/process");
    assert_eq!(
        body.get("rrr").and_then(Value::as_str),
        Some("141523991664")
    );
    assert_eq!(body.get("amount").and_then(Value::as_i64), Some(1000));
    assert_eq!(
        body.get("paymentIdentifier").and_then(Value::as_str),
        Some(ORDER_ID)
    );
}

#[test]
fn webhook_headers_are_checked() {
    let body = br#"{"request_id":"abc","status":"PENDING"}"#;

    let inlomax = InlomaxClient::new(
        String::from("https://inlomax.example"),
        SecretString::from(String::from("inlomax-api")),
        SecretString::from(String::from("inlomax-webhook")),
        Duration::from_secs(5),
    )
    .expect("inlomax client");
    let inlomax_signature = hmac_hex("inlomax-webhook", body);
    assert!(inlomax.verify_webhook(&header_map("x-inlomax-signature", &inlomax_signature), body));
    assert!(!inlomax.verify_webhook(&header_map("x-inlomax-signature", "wrong"), body));
    assert!(!inlomax.verify_webhook(&HeaderMap::new(), body));

    let vtpass = VtpassClient::new(
        String::from("https://vtpass.example"),
        SecretString::from(String::from("api")),
        SecretString::from(String::from("secret")),
        SecretString::from(String::from("vtpass-webhook")),
        Duration::from_secs(5),
    )
    .expect("vtpass client");
    let vtpass_signature = hmac_hex("vtpass-webhook", body);
    assert!(vtpass.verify_webhook(&header_map("x-vtpass-signature", &vtpass_signature), body));
    assert!(!vtpass.verify_webhook(&header_map("x-vtpass-signature", "wrong"), body));
    assert!(!vtpass.verify_webhook(&HeaderMap::new(), body));
}

#[test]
fn remita_webhook_verification_accepts_unsigned_callbacks() {
    let remita = RemitaClient::new(
        String::from("https://remita.example"),
        String::from("ZUTILITY"),
        SecretString::from(String::from("remita-api-key")),
        String::from("4430731"),
        SecretString::from(String::from("remita-webhook")),
        Duration::from_secs(5),
    )
    .expect("remita client");

    let collections = RemitaCollectionsClient::new(
        String::from("https://remita-collections.example"),
        SecretString::from(String::from("remita-collections-key")),
        Duration::from_secs(5),
    )
    .expect("remita collections client");

    assert!(remita.verify_webhook(&HeaderMap::new(), b"{}"));
    assert!(collections.verify_webhook(&HeaderMap::new(), b"{}"));
}
