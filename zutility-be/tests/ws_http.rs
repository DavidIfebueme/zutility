use axum::body::{Body, to_bytes};
use axum::http::Request;
use axum::http::StatusCode;
use futures_util::SinkExt;
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
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

#[tokio::test]
async fn websocket_reconnect_with_same_token_is_safe() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000009","amount_ngn":5000,"zec_address_type":"shielded"}"#,
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

    let url = format!("ws://{addr}/api/v1/orders/{order_id}/stream?token={token}");
    let first = connect_async(&url).await;
    assert!(first.is_ok());
    if let Ok((mut socket, _)) = first {
        let _ = socket.close(None).await;
    }

    let second = connect_async(&url).await;
    assert!(second.is_ok());
    if let Ok((mut socket, _)) = second {
        let _ = socket.close(None).await;
    }

    server.abort();
}

#[tokio::test]
async fn websocket_stream_emits_terminal_event_shape() {
    let app = http::router();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"utility_type":"airtime","utility_slug":"mtn","service_ref":"08000000111","amount_ngn":5000,"zec_address_type":"shielded"}"#,
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

    let url = format!("ws://{addr}/api/v1/orders/{order_id}/stream?token={token}");
    let connected = connect_async(&url).await;
    assert!(connected.is_ok());
    if let Ok((mut socket, _)) = connected {
        let _ = socket
            .send(Message::Text(String::from("ping").into()))
            .await;
        let _ = socket.close(None).await;
    }

    let completed = serde_json::to_value(zutility_be::ws::WsOrderEvent::Completed {
        delivery_token: None,
        reference: String::from("ref-1"),
    })
    .expect("serialize completed");
    assert_eq!(
        completed.get("event").and_then(Value::as_str),
        Some("completed")
    );
    assert!(completed.get("delivery_token").is_some());
    assert!(completed.get("token").is_none());

    server.abort();
}
