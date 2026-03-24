use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::http::StatusCode;
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tower::util::ServiceExt;
use zutility_be::http;

#[tokio::test]
async fn websocket_handshake_requires_valid_order_token() {
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
    let create_json: Value = serde_json::from_slice(&create_body).expect("parse create response");
    let order_id = create_json
        .get("order_id")
        .and_then(Value::as_str)
        .expect("order id in response");
    let token = create_json
        .get("order_access_token")
        .and_then(Value::as_str)
        .expect("token in response");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let invalid_url = format!("ws://{addr}/api/v1/orders/{order_id}/stream?token=wrong");
    let invalid_result = connect_async(&invalid_url).await;
    assert!(invalid_result.is_err());

    let valid_url = format!("ws://{addr}/api/v1/orders/{order_id}/stream?token={token}");
    let valid_result = connect_async(&valid_url).await;
    assert!(valid_result.is_ok());

    if let Ok((mut socket, _)) = valid_result {
        let _ = socket.close(None).await;
    }

    server.abort();
}
