use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;
use zutility_be::integrations::{
    utility_provider::{ProviderTxnStatus, UtilityProvider, UtilityPurchaseRequest},
    vtpass::{RetryPolicy, VtpassClient},
};

#[derive(Clone, Default)]
struct MockVtpassState {
    pay_calls: Arc<Mutex<u32>>,
    requery_calls: Arc<Mutex<u32>>,
    slow_first_pay: bool,
}

async fn pay_handler(
    State(state): State<MockVtpassState>,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let mut calls = state.pay_calls.lock().await;
    *calls += 1;

    if state.slow_first_pay && *calls == 1 {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let request_id = payload
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(json!({
            "code": "000",
            "response_description": "TRANSACTION SUCCESSFUL",
            "request_id": request_id,
            "content": {
                "transactionId": "tx-abc"
            },
            "token": "token-xyz"
        })),
    )
}

async fn requery_handler(State(state): State<MockVtpassState>) -> (StatusCode, Json<Value>) {
    let mut calls = state.requery_calls.lock().await;
    *calls += 1;

    (
        StatusCode::OK,
        Json(json!({
            "code": "000",
            "response_description": "TRANSACTION SUCCESSFUL",
            "request_id": "req-1",
            "token": "token-xyz"
        })),
    )
}

async fn spawn_mock_vtpass(state: MockVtpassState) -> String {
    let app = Router::new()
        .route("/pay", post(pay_handler))
        .route("/requery", get(requery_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock vtpass");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    format!("http://{addr}")
}

fn build_purchase_request() -> UtilityPurchaseRequest {
    UtilityPurchaseRequest {
        order_id: Uuid::new_v4(),
        request_id: String::from("req-1"),
        service_id: String::from("mtn"),
        billers_code: String::from("08000000000"),
        variation_code: None,
        amount_ngn: 1000,
        phone: Some(String::from("08000000000")),
        metadata: json!({}),
        zec_amount: rust_decimal::Decimal::new(1, 0),
    }
}

#[tokio::test]
async fn vtpass_pay_and_requery_happy_path() {
    let state = MockVtpassState::default();
    let base_url = spawn_mock_vtpass(state.clone()).await;
    let client = VtpassClient::new(
        base_url,
        secrecy::SecretString::from(String::from("api")),
        secrecy::SecretString::from(String::from("secret")),
        secrecy::SecretString::from(String::from("webhook")),
        Duration::from_millis(100),
    )
    .expect("build client")
    .with_retry_policy(RetryPolicy {
        max_attempts: 2,
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(20),
    });

    let pay = client.pay(&build_purchase_request()).await.expect("pay ok");
    assert_eq!(pay.status, ProviderTxnStatus::Delivered);
    assert_eq!(pay.provider_request_id, "req-1");

    let requery = client.requery("req-1").await.expect("requery ok");
    assert_eq!(requery.status, ProviderTxnStatus::Delivered);
    assert_eq!(requery.provider_request_id, "req-1");

    assert_eq!(*state.pay_calls.lock().await, 1);
    assert_eq!(*state.requery_calls.lock().await, 1);
}

#[tokio::test]
async fn vtpass_pending_to_delivered_callback_path() {
    let client = VtpassClient::new(
        String::from("https://sandbox.vtpass.com/api"),
        secrecy::SecretString::from(String::from("api")),
        secrecy::SecretString::from(String::from("secret")),
        secrecy::SecretString::from(String::from("webhook")),
        Duration::from_millis(200),
    )
    .expect("build client");

    let pending_payload = br#"{"request_id":"req-1","status":"PENDING","code":"099"}"#;
    let delivered_payload =
        br#"{"request_id":"req-1","status":"TRANSACTION SUCCESSFUL","code":"000","token":"12345"}"#;

    let pending = client
        .parse_webhook_event(pending_payload)
        .expect("parse pending");
    let delivered = client
        .parse_webhook_event(delivered_payload)
        .expect("parse delivered");

    assert_eq!(pending.status, ProviderTxnStatus::Pending);
    assert_eq!(delivered.status, ProviderTxnStatus::Delivered);
    assert_eq!(delivered.token.as_deref(), Some("12345"));
}

#[tokio::test]
async fn vtpass_timeout_retries_and_succeeds() {
    let state = MockVtpassState {
        pay_calls: Arc::new(Mutex::new(0)),
        requery_calls: Arc::new(Mutex::new(0)),
        slow_first_pay: true,
    };

    let base_url = spawn_mock_vtpass(state.clone()).await;
    let client = VtpassClient::new(
        base_url,
        secrecy::SecretString::from(String::from("api")),
        secrecy::SecretString::from(String::from("secret")),
        secrecy::SecretString::from(String::from("webhook")),
        Duration::from_millis(50),
    )
    .expect("build client")
    .with_retry_policy(RetryPolicy {
        max_attempts: 2,
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(20),
    });

    let pay = client.pay(&build_purchase_request()).await.expect("pay ok");
    assert_eq!(pay.status, ProviderTxnStatus::Delivered);

    let calls = *state.pay_calls.lock().await;
    assert_eq!(calls, 2);
}
