use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
use zutility_be::{
    db::{
        CreateOrderInput, OrderStatusTransitionRecord, apply_order_status_transition, begin_tx,
        insert_order_with_claimed_address,
    },
    domain::order::{OrderStatus, OrderStatusTransition},
};

async fn test_pool() -> Option<PgPool> {
    let database_url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .ok()?;
    Some(pool)
}

async fn prepare_schema(pool: &PgPool) {
    if sqlx::migrate!("./migrations").run(pool).await.is_err() {
        return;
    }

    let _ =
        sqlx::query("TRUNCATE TABLE audit_log, orders, deposit_addresses, rate_snapshots CASCADE")
            .execute(pool)
            .await;

    let _ = sqlx::query(
        "INSERT INTO rate_snapshots (id, zec_ngn, zec_usd, usd_ngn, sources_used, fetched_at)
         VALUES ($1, 1000000, 50, 2000, $2, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(vec![String::from("test")])
    .bind(Utc::now())
    .execute(pool)
    .await;
}

#[tokio::test]
async fn atomic_transition_updates_order_and_audit_log() {
    let Some(pool) = test_pool().await else {
        return;
    };

    prepare_schema(&pool).await;

    let rate_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM rate_snapshots LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("rate snapshot should exist");

    let address = format!("ztestsapling1{}", Uuid::new_v4().simple());
    let _ = sqlx::query(
        "INSERT INTO deposit_addresses (address, address_type, used) VALUES ($1, 'shielded', false)",
    )
    .bind(address)
    .execute(&pool)
    .await
    .expect("insert deposit address");

    let mut tx = begin_tx(&pool).await.expect("begin tx");
    let order_id = insert_order_with_claimed_address(
        &mut tx,
        &CreateOrderInput {
            access_token_hash: String::from("token-hash"),
            utility_type: String::from("airtime"),
            utility_slug: String::from("mtn"),
            service_ref: String::from("08000000000"),
            amount_ngn: 1000,
            address_type: String::from("shielded"),
            zec_amount: Decimal::new(1, 0),
            zec_rate_id: rate_id,
            required_confs: 10,
            expires_at: Utc::now() + Duration::minutes(20),
            ip_hash: Some(String::from("hash")),
            metadata: serde_json::json!({"source":"test"}),
        },
    )
    .await
    .expect("insert order");

    let transition =
        OrderStatusTransition::new(OrderStatus::AwaitingPayment, OrderStatus::PaymentDetected)
            .expect("valid transition");
    apply_order_status_transition(
        &mut tx,
        &OrderStatusTransitionRecord {
            order_id,
            transition,
            event: String::from("payment_detected"),
            detail: serde_json::json!({"confirmations": 1}),
        },
    )
    .await
    .expect("apply transition");

    tx.commit().await.expect("commit tx");

    let status = sqlx::query_scalar::<_, String>("SELECT status FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("fetch status");
    assert_eq!(status, "payment_detected");

    let audit_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_log WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(&pool)
            .await
            .expect("fetch audit count");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn guarded_transition_fails_when_current_status_mismatches() {
    let Some(pool) = test_pool().await else {
        return;
    };

    prepare_schema(&pool).await;

    let rate_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM rate_snapshots LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("rate snapshot should exist");

    let address = format!("ztestsapling1{}", Uuid::new_v4().simple());
    let _ = sqlx::query(
        "INSERT INTO deposit_addresses (address, address_type, used) VALUES ($1, 'shielded', false)",
    )
    .bind(address)
    .execute(&pool)
    .await
    .expect("insert deposit address");

    let mut tx = begin_tx(&pool).await.expect("begin tx");
    let order_id = insert_order_with_claimed_address(
        &mut tx,
        &CreateOrderInput {
            access_token_hash: String::from("token-hash"),
            utility_type: String::from("airtime"),
            utility_slug: String::from("mtn"),
            service_ref: String::from("08011111111"),
            amount_ngn: 1500,
            address_type: String::from("shielded"),
            zec_amount: Decimal::new(1, 0),
            zec_rate_id: rate_id,
            required_confs: 10,
            expires_at: Utc::now() + Duration::minutes(20),
            ip_hash: Some(String::from("hash")),
            metadata: serde_json::json!({"source":"test"}),
        },
    )
    .await
    .expect("insert order");

    let wrong_from =
        OrderStatusTransition::new(OrderStatus::PaymentDetected, OrderStatus::PaymentConfirmed)
            .expect("transition shape is valid");
    let result = apply_order_status_transition(
        &mut tx,
        &OrderStatusTransitionRecord {
            order_id,
            transition: wrong_from,
            event: String::from("invalid_transition"),
            detail: serde_json::json!({}),
        },
    )
    .await;

    assert!(result.is_err());
    tx.rollback().await.expect("rollback tx");

    let order_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("fetch order count");
    assert_eq!(order_count, 0);
}
