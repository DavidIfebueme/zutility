use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::util::ServiceExt;
use zutility_be::http;

#[tokio::test]
async fn create_and_get_order_with_valid_token() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000000","amount_ngn":5000,"zec_address_type":"shielded"}"#,
        ))
        .expect("build request");

    let create_res = app.clone().oneshot(create_req).await.expect("response");
    assert_eq!(create_res.status(), StatusCode::OK);

    let create_body = to_bytes(create_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let create_json: Value = serde_json::from_slice(&create_body).expect("json parse");

    let order_id = create_json
        .get("order_id")
        .and_then(Value::as_str)
        .expect("order id in response");
    let token = create_json
        .get("order_access_token")
        .and_then(Value::as_str)
        .expect("token in response");

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orders/{order_id}?token={token}"))
        .body(Body::empty())
        .expect("build get request");

    let get_res = app.oneshot(get_req).await.expect("get response");
    assert_eq!(get_res.status(), StatusCode::OK);

    let get_body = to_bytes(get_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let get_json: Value = serde_json::from_slice(&get_body).expect("json parse");

    assert_eq!(
        get_json.get("status").and_then(Value::as_str),
        Some("awaiting_payment")
    );
}

#[tokio::test]
async fn get_order_with_invalid_token_is_forbidden() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000000","amount_ngn":5000,"zec_address_type":"transparent"}"#,
        ))
        .expect("build request");

    let create_res = app.clone().oneshot(create_req).await.expect("response");
    let create_body = to_bytes(create_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let create_json: Value = serde_json::from_slice(&create_body).expect("json parse");
    let order_id = create_json
        .get("order_id")
        .and_then(Value::as_str)
        .expect("order id in response");

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orders/{order_id}?token=wrong"))
        .body(Body::empty())
        .expect("build request");

    let get_res = app.oneshot(get_req).await.expect("response");
    assert_eq!(get_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cancel_order_changes_status() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"airtel","service_ref":"08011111111","amount_ngn":7000,"zec_address_type":"transparent"}"#,
        ))
        .expect("build request");

    let create_res = app.clone().oneshot(create_req).await.expect("response");
    let create_body = to_bytes(create_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let create_json: Value = serde_json::from_slice(&create_body).expect("json parse");

    let order_id = create_json
        .get("order_id")
        .and_then(Value::as_str)
        .expect("order id in response");
    let token = create_json
        .get("order_access_token")
        .and_then(Value::as_str)
        .expect("token in response");

    let cancel_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/orders/{order_id}/cancel?token={token}"))
        .body(Body::empty())
        .expect("build request");

    let cancel_res = app.clone().oneshot(cancel_req).await.expect("response");
    assert_eq!(cancel_res.status(), StatusCode::OK);

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/orders/{order_id}?token={token}"))
        .body(Body::empty())
        .expect("build request");

    let get_res = app.oneshot(get_req).await.expect("response");
    let get_body = to_bytes(get_res.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let get_json: Value = serde_json::from_slice(&get_body).expect("json parse");
    assert_eq!(
        get_json.get("status").and_then(Value::as_str),
        Some("cancelled")
    );
}

#[tokio::test]
async fn rates_utilities_and_validate_endpoints_respond() {
    let app = http::router();

    let rates_req = Request::builder()
        .method("GET")
        .uri("/api/v1/rates/current")
        .body(Body::empty())
        .expect("build request");
    let rates_res = app.clone().oneshot(rates_req).await.expect("response");
    assert_eq!(rates_res.status(), StatusCode::OK);

    let utilities_req = Request::builder()
        .method("GET")
        .uri("/api/v1/utilities")
        .body(Body::empty())
        .expect("build request");
    let utilities_res = app.clone().oneshot(utilities_req).await.expect("response");
    assert_eq!(utilities_res.status(), StatusCode::OK);

    let validate_req = Request::builder()
        .method("GET")
        .uri("/api/v1/utilities/mtn/validate?ref=08000000000")
        .body(Body::empty())
        .expect("build request");
    let validate_res = app.oneshot(validate_req).await.expect("response");
    assert_eq!(validate_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn service_ref_velocity_limit_returns_too_many_requests() {
    let app = http::router();

    for _ in 0..8 {
        let create_req = Request::builder()
            .method("POST")
            .uri("/api/v1/orders/create")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08099999999","amount_ngn":5000,"zec_address_type":"transparent"}"#,
            ))
            .expect("build request");
        let create_res = app.clone().oneshot(create_req).await.expect("response");
        assert_eq!(create_res.status(), StatusCode::OK);
    }

    let blocked_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08099999999","amount_ngn":5000,"zec_address_type":"transparent"}"#,
        ))
        .expect("build request");
    let blocked_res = app.oneshot(blocked_req).await.expect("response");
    assert_eq!(blocked_res.status(), StatusCode::TOO_MANY_REQUESTS);
}
