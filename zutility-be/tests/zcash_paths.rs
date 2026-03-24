use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use rust_decimal::Decimal;
use serde_json::Value;
use tower::util::ServiceExt;
use zutility_be::{
    http,
    integrations::zcash::{PaymentMatchStatus, ReceivedNote, evaluate_received_notes},
};

#[tokio::test]
async fn transparent_payment_path_uses_three_confirmations() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000001","amount_ngn":5000,"zec_address_type":"transparent"}"#,
        ))
        .expect("build request");

    let create_res = app.oneshot(create_req).await.expect("response");
    assert_eq!(create_res.status(), StatusCode::OK);

    let body = to_bytes(create_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let json: Value = serde_json::from_slice(&body).expect("json parse");
    assert_eq!(
        json.get("required_confirmations").and_then(Value::as_u64),
        Some(3)
    );
}

#[tokio::test]
async fn shielded_payment_path_uses_ten_confirmations() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000002","amount_ngn":5000,"zec_address_type":"shielded"}"#,
        ))
        .expect("build request");

    let create_res = app.oneshot(create_req).await.expect("response");
    assert_eq!(create_res.status(), StatusCode::OK);

    let body = to_bytes(create_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let json: Value = serde_json::from_slice(&body).expect("json parse");
    assert_eq!(
        json.get("required_confirmations").and_then(Value::as_u64),
        Some(10)
    );
}

#[test]
fn underpayment_and_overpayment_paths_are_classified() {
    let notes = vec![
        ReceivedNote {
            txid: String::from("tx-1"),
            address: String::from("ztestsapling1abc"),
            amount: Decimal::new(5, 1),
            confirmations: 4,
            memo: None,
        },
        ReceivedNote {
            txid: String::from("tx-2"),
            address: String::from("ztestsapling1abc"),
            amount: Decimal::new(5, 1),
            confirmations: 4,
            memo: None,
        },
    ];

    let underpaid = evaluate_received_notes(&notes, Decimal::new(11, 1));
    assert_eq!(underpaid.status, PaymentMatchStatus::Underpaid);

    let overpaid = evaluate_received_notes(&notes, Decimal::new(9, 1));
    assert_eq!(overpaid.status, PaymentMatchStatus::Overpaid);
}
