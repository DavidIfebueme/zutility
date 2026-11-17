#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use rust_decimal::Decimal;
use secrecy::SecretString;
use serde_json::Value;
use sqlx::PgPool;
use tower::util::ServiceExt;
use uuid::Uuid;
use zutility_be::{
    domain::money::{CurrencyCode, Money},
    http::{self, types::DisplayMoney},
};

mod support;

async fn session_with_currency(pool: &PgPool, currency: &str) -> support::Session {
    let email = format!("fx-{}@example.com", Uuid::new_v4());
    let user = zutility_be::db::create_user(pool, &email, Some("fx"), "hash", currency)
        .await
        .expect("create user");
    let csrf = zutility_be::http::auth::generate_csrf_token();
    let secret = SecretString::from(http::DEV_JWT_SECRET.to_string());
    let jwt = zutility_be::http::auth::create_access_jwt(user.id, &user.email, &csrf, &secret, 15)
        .expect("access jwt");

    support::Session { jwt, csrf }
}

async fn seed_usd_ngn(pool: &PgPool, usd_ngn: Decimal) {
    sqlx::query(
        "INSERT INTO rate_snapshots (id, zec_ngn, zec_usd, usd_ngn, sources_used, fetched_at)
         VALUES ($1, 1000000, 50, $2, $3, now())",
    )
    .bind(Uuid::new_v4())
    .bind(usd_ngn)
    .bind(vec![String::from("test")])
    .execute(pool)
    .await
    .expect("seed rate snapshot");
}

async fn seed_fx(pool: &PgPool, quote: &str, per_usd: Decimal) {
    sqlx::query(
        "INSERT INTO fx_rates (quote, per_usd, source, fetched_at)
         VALUES ($1, $2, 'test', now())
         ON CONFLICT (quote) DO UPDATE
         SET per_usd = EXCLUDED.per_usd,
             source = EXCLUDED.source,
             fetched_at = EXCLUDED.fetched_at",
    )
    .bind(quote)
    .bind(per_usd)
    .execute(pool)
    .await
    .expect("seed fx rate");
}

async fn clear_fx(pool: &PgPool) {
    sqlx::query("DELETE FROM fx_rates")
        .execute(pool)
        .await
        .expect("clear fx rates");
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("build request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");

    (status, serde_json::from_slice(&body).expect("json body"))
}

async fn create_order(app: &Router, amount_ngn: i64, service_ref: &str) -> Value {
    let payload = serde_json::json!({
        "utility_type": "airtime",
        "utility_slug": "mtn",
        "service_ref": service_ref,
        "amount_ngn": amount_ngn,
        "zec_address_type": "shielded",
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/orders/create")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build request");
    let response = app.clone().oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");

    serde_json::from_slice(&body).expect("json body")
}

fn display_amount(body: &Value) -> Option<Decimal> {
    body.get("display")?
        .get("amount")?
        .as_str()?
        .parse::<Decimal>()
        .ok()
}

fn display_currency(body: &Value) -> Option<&str> {
    body.get("display")?.get("currency")?.as_str()
}

fn rate_available(body: &Value) -> Option<bool> {
    body.get("display")?.get("rate_available")?.as_bool()
}

async fn history_display(pool: &PgPool, currency: &str, amount_ngn: i64, uri: &str) -> Value {
    clear_fx(pool).await;
    seed_usd_ngn(pool, Decimal::from(1500)).await;
    seed_fx(pool, "XOF", Decimal::from(600)).await;
    seed_fx(pool, "ZAR", Decimal::from(18)).await;

    let session = session_with_currency(pool, currency).await;
    let app = support::authed_app(pool.clone(), &session);
    let _ = create_order(&app, amount_ngn, "08000000001").await;

    let (status, body) = get_json(&app, uri).await;
    assert_eq!(status, StatusCode::OK);
    body.as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("display"))
        .cloned()
        .expect("display block on history item")
}

#[tokio::test]
async fn ngn_to_xof_projects_through_the_bridge() {
    let Some(pool) = support::pool().await else {
        return;
    };

    let display = history_display(&pool, "XOF", 1000, "/api/v1/orders/history").await;

    assert_eq!(display.get("currency").and_then(Value::as_str), Some("XOF"));
    assert_eq!(
        display.get("rate_available").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        display
            .get("amount")
            .and_then(Value::as_str)
            .map(|raw| raw.parse::<Decimal>().expect("decimal amount")),
        Some(Decimal::from(400))
    );
}

#[tokio::test]
async fn ngn_to_zar_projects_through_the_bridge() {
    let Some(pool) = support::pool().await else {
        return;
    };

    let display = history_display(&pool, "ZAR", 3000, "/api/v1/orders/history").await;

    assert_eq!(display.get("currency").and_then(Value::as_str), Some("ZAR"));
    assert_eq!(
        display.get("rate_available").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        display
            .get("amount")
            .and_then(Value::as_str)
            .map(|raw| raw.parse::<Decimal>().expect("decimal amount")),
        Some(Decimal::from(36))
    );
}

#[tokio::test]
async fn ngn_to_usd_uses_the_bridge_rate() {
    let Some(pool) = support::pool().await else {
        return;
    };

    let display = history_display(&pool, "USD", 3000, "/api/v1/orders/history").await;

    assert_eq!(display.get("currency").and_then(Value::as_str), Some("USD"));
    assert_eq!(
        display.get("rate_available").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        display
            .get("amount")
            .and_then(Value::as_str)
            .map(|raw| raw.parse::<Decimal>().expect("decimal amount")),
        Some(Decimal::from(2))
    );
}

#[tokio::test]
async fn query_currency_overrides_the_profile() {
    let Some(pool) = support::pool().await else {
        return;
    };

    let display = history_display(&pool, "ZAR", 1000, "/api/v1/orders/history?currency=XOF").await;

    assert_eq!(display.get("currency").and_then(Value::as_str), Some("XOF"));
    assert_eq!(
        display
            .get("amount")
            .and_then(Value::as_str)
            .map(|raw| raw.parse::<Decimal>().expect("decimal amount")),
        Some(Decimal::from(400))
    );
}

#[tokio::test]
async fn missing_fx_row_returns_unavailable() {
    let Some(pool) = support::pool().await else {
        return;
    };

    clear_fx(&pool).await;
    seed_usd_ngn(&pool, Decimal::from(1500)).await;

    let session = session_with_currency(&pool, "XOF").await;
    let app = support::authed_app(pool.clone(), &session);
    let _ = create_order(&app, 1000, "08000000002").await;

    let (status, body) = get_json(&app, "/api/v1/orders/history").await;
    assert_eq!(status, StatusCode::OK);

    let item = body
        .as_array()
        .and_then(|items| items.first())
        .expect("history item");

    assert_eq!(rate_available(item), Some(false));
    assert_eq!(display_currency(item), Some("XOF"));
    assert_eq!(display_amount(item), None);
    assert_eq!(item.get("amount_ngn").and_then(Value::as_i64), Some(1000));
}

#[test]
fn zero_rate_returns_unavailable_instead_of_dividing() {
    let naira = Money::from_ngn_whole(1000).expect("naira");

    let zero_bridge = DisplayMoney::new(
        CurrencyCode::Xof,
        Some(naira),
        Some((Decimal::ZERO, Decimal::from(600))),
    );
    let zero_target = DisplayMoney::new(
        CurrencyCode::Xof,
        Some(naira),
        Some((Decimal::from(1500), Decimal::ZERO)),
    );

    for display in [zero_bridge, zero_target] {
        assert!(!display.rate_available);
        assert_eq!(display.amount, None);
        assert_eq!(display.currency, String::from("XOF"));
    }

    assert_eq!(
        naira.display_in(CurrencyCode::Xof, Decimal::ZERO, Decimal::from(600)),
        None
    );
    assert_eq!(
        naira.display_in(CurrencyCode::Xof, Decimal::from(1500), Decimal::ZERO),
        None
    );
}

#[test]
fn sub_unit_conversion_rounds_half_up() {
    let half = Money::from_ngn_whole(1000).expect("naira");
    let two_and_a_half = Money::from_ngn_whole(2500).expect("naira");

    let rounded_up = half
        .display_in(CurrencyCode::Xof, Decimal::from(1000), Decimal::new(15, 1))
        .expect("converted");
    let rounded_from_midpoint = two_and_a_half
        .display_in(CurrencyCode::Xof, Decimal::from(1000), Decimal::ONE)
        .expect("converted");

    assert_eq!(rounded_up.round_to_currency(), Decimal::from(2));
    assert_eq!(rounded_from_midpoint.round_to_currency(), Decimal::from(3));
}

#[tokio::test]
async fn every_money_dto_carries_the_display_block() {
    let Some(pool) = support::pool().await else {
        return;
    };

    clear_fx(&pool).await;
    seed_usd_ngn(&pool, Decimal::from(1500)).await;
    for (quote, per_usd) in [
        ("XOF", 600),
        ("GMD", 68),
        ("MWK", 1730),
        ("ZMW", 26),
        ("ZAR", 18),
        ("KES", 129),
        ("GHS", 12),
        ("EGP", 48),
    ] {
        seed_fx(&pool, quote, Decimal::from(per_usd)).await;
    }

    let session = session_with_currency(&pool, "XOF").await;
    let app = support::authed_app(pool.clone(), &session);

    let created = create_order(&app, 1000, "08000000003").await;
    let order_id = created
        .get("order_id")
        .and_then(Value::as_str)
        .expect("order id");
    let token = created
        .get("order_access_token")
        .and_then(Value::as_str)
        .expect("order token");

    let (status, order) = get_json(&app, &format!("/api/v1/orders/{order_id}?token={token}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(display_currency(&order), Some("XOF"));
    assert_eq!(display_amount(&order), Some(Decimal::from(400)));

    let (status, history) = get_json(&app, "/api/v1/orders/history").await;
    assert_eq!(status, StatusCode::OK);
    let history_item = history
        .as_array()
        .and_then(|items| items.first())
        .expect("history item");
    assert_eq!(display_currency(history_item), Some("XOF"));

    let gift_payload = serde_json::json!({
        "utility_type": "airtime",
        "utility_slug": "mtn",
        "amount_ngn": 5000,
        "zec_address_type": "shielded",
        "claim_window_hours": 24,
    });
    let gift_request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts")
        .header("content-type", "application/json")
        .body(Body::from(gift_payload.to_string()))
        .expect("build request");
    let gift_response = app.clone().oneshot(gift_request).await.expect("response");
    assert_eq!(gift_response.status(), StatusCode::OK);
    let gift_body = to_bytes(gift_response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let gift: Value = serde_json::from_slice(&gift_body).expect("json body");
    let gift_code = gift
        .get("gift_code")
        .and_then(Value::as_str)
        .expect("gift code");

    let (status, summaries) = get_json(&app, "/api/v1/gifts").await;
    assert_eq!(status, StatusCode::OK);
    let summary = summaries
        .as_array()
        .and_then(|items| items.first())
        .expect("gift summary");
    assert_eq!(display_currency(summary), Some("XOF"));
    assert_eq!(rate_available(summary), Some(true));

    let (status, view) = get_json(&app, &format!("/api/v1/gifts/{gift_code}?currency=XOF")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(display_currency(&view), Some("XOF"));
    assert_eq!(rate_available(&view), Some(true));

    let (status, rates) = get_json(&app, "/api/v1/rates/current?currency=XOF").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        rates.get("fx").and_then(Value::as_array).map(Vec::len),
        Some(8)
    );
    assert_eq!(display_currency(&rates), Some("XOF"));
    assert_eq!(rate_available(&rates), Some(true));

    let (status, validated) = get_json(
        &app,
        "/api/v1/utilities/mtn/validate?ref=08000000003&currency=XOF",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(display_currency(&validated), Some("XOF"));
    assert_eq!(rate_available(&validated), Some(false));
    assert_eq!(display_amount(&validated), None);
}
