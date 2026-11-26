#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use rust_decimal::Decimal;
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;
use zutility_be::integrations::{
    remita::RemitaClient,
    remita_collections::RemitaCollectionsClient,
    utility_provider::{ProviderErrorKind, UtilityProvider, UtilityPurchaseRequest},
};

const ORDER_ID: &str = "00000000-0000-4000-8000-000000000001";

#[derive(Clone, Default)]
struct Mock {
    requests: Arc<Mutex<Vec<(String, Value)>>>,
    lookup_amount: Arc<Mutex<Option<i64>>>,
}

impl Mock {
    async fn recorded(&self) -> Vec<(String, Value)> {
        self.requests.lock().await.clone()
    }

    async fn set_lookup_amount(&self, amount: i64) {
        *self.lookup_amount.lock().await = Some(amount);
    }
}

async fn lookup_handler(State(state): State<Mock>, Path(rrr): Path<String>) -> Json<Value> {
    let path = format!("/remita-collections/biller/lookup/{rrr}");
    let amount = state.lookup_amount.lock().await.unwrap_or(1000);
    state.requests.lock().await.push((path, Value::Null));

    Json(json!({
        "status": "00",
        "message": "ok",
        "data": {
            "rrr": rrr,
            "amount": amount,
            "fee": 0,
            "status": "pending",
            "billerName": "NSITF",
            "payerName": "Test Payer"
        }
    }))
}

async fn process_handler(State(state): State<Mock>, Json(body): Json<Value>) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((String::from("/remita-collections/biller/process"), body));

    Json(json!({
        "status": "00",
        "message": "ok",
        "data": {"transactionStatus": "success"}
    }))
}

async fn billpayment_handler(State(state): State<Mock>, Json(body): Json<Value>) -> Json<Value> {
    state
        .requests
        .lock()
        .await
        .push((String::from("/remita/billpayment/pay"), body));

    Json(json!({"status_code": "021", "status": "00", "rrr": "rrr-1"}))
}

async fn auth_failure_handler() -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"message": "unauthorized"})),
    )
}

async fn spawn_server() -> (Mock, String) {
    let state = Mock::default();
    let app = Router::new()
        .route(
            "/remita-collections/biller/lookup/{rrr}",
            get(lookup_handler),
        )
        .route("/remita-collections/biller/process", post(process_handler))
        .route("/remita/billpayment/pay", post(billpayment_handler))
        .route("/remita-auth/billpayment/pay", post(auth_failure_handler))
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

fn request(amount_ngn: i64, metadata: Value) -> UtilityPurchaseRequest {
    UtilityPurchaseRequest {
        order_id: Uuid::parse_str(ORDER_ID).expect("order id"),
        request_id: String::from("req-1"),
        service_id: String::from("school-fees"),
        billers_code: String::from("141523991664"),
        variation_code: None,
        amount_ngn,
        phone: None,
        metadata,
        zec_amount: Decimal::new(1, 0),
    }
}

fn collections_client(base_url: &str) -> RemitaCollectionsClient {
    RemitaCollectionsClient::new(
        base_url.to_owned(),
        SecretString::from(String::from("remita-collections-key")),
        Duration::from_secs(5),
    )
    .expect("remita collections client")
}

fn billpayment_client(base_url: &str) -> RemitaClient {
    RemitaClient::new(
        base_url.to_owned(),
        String::from("ZUTILITY"),
        SecretString::from(String::from("remita-api-key")),
        String::from("4430731"),
        SecretString::from(String::from("remita-webhook")),
        Duration::from_secs(5),
    )
    .expect("remita client")
}

#[tokio::test]
async fn collections_rejects_a_settlement_that_differs_from_the_rrr() {
    let (server, base_url) = spawn_server().await;
    server.set_lookup_amount(900).await;

    let error = collections_client(&format!("{base_url}/remita-collections"))
        .pay(&request(1000, json!({})))
        .await
        .expect_err("settlement mismatch");

    assert!(matches!(error.kind, ProviderErrorKind::Permanent));
    let recorded = server.recorded().await;
    assert!(
        !recorded
            .iter()
            .any(|(path, _)| path.ends_with("/biller/process"))
    );
}

#[tokio::test]
async fn collections_processes_the_rrr_amount() {
    let (server, base_url) = spawn_server().await;
    server.set_lookup_amount(1200).await;

    collections_client(&format!("{base_url}/remita-collections"))
        .pay(&request(1200, json!({})))
        .await
        .expect("collections pay");

    let recorded = server.recorded().await;
    let (path, body) = recorded.last().expect("process request");
    assert_eq!(path, "/remita-collections/biller/process");
    assert_eq!(body.get("amount").and_then(Value::as_i64), Some(1200));
}

#[tokio::test]
async fn billpayment_omits_a_missing_payer_phone() {
    let (server, base_url) = spawn_server().await;

    billpayment_client(&format!("{base_url}/remita"))
        .pay(&request(1000, json!({})))
        .await
        .expect("billpayment pay");

    let recorded = server.recorded().await;
    let (_, body) = recorded.last().expect("pay request");
    assert!(body.get("payerPhone").is_none());
}

#[tokio::test]
async fn billpayment_forwards_a_supplied_payer_phone() {
    let (server, base_url) = spawn_server().await;

    billpayment_client(&format!("{base_url}/remita"))
        .pay(&request(1000, json!({"payer_phone": "08030000000"})))
        .await
        .expect("billpayment pay");

    let recorded = server.recorded().await;
    let (_, body) = recorded.last().expect("pay request");
    assert_eq!(
        body.get("payerPhone").and_then(Value::as_str),
        Some("08030000000")
    );
}

#[tokio::test]
async fn auth_failures_do_not_open_the_breaker() {
    let (_, base_url) = spawn_server().await;
    let client = billpayment_client(&format!("{base_url}/remita-auth"));

    for _ in 0..7 {
        let error = client
            .pay(&request(1000, json!({})))
            .await
            .expect_err("auth failure");

        assert!(matches!(error.kind, ProviderErrorKind::Unauthorized));
    }
}
