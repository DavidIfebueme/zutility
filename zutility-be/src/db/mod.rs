use crate::domain::order::{OrderStatus, OrderStatusTransition};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbProvider {
    Postgres,
}

#[derive(Debug, Clone)]
pub struct OrderStatusTransitionRecord {
    pub order_id: Uuid,
    pub transition: OrderStatusTransition,
    pub event: String,
    pub detail: Value,
}

#[derive(Debug, Clone)]
pub struct WebhookTransition {
    pub from_status: String,
    pub to_status: String,
    pub event: String,
    pub delivery_token: Option<String>,
    pub provider_reference: Option<String>,
    pub detail: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookTransitionOutcome {
    Applied,
    Recorded,
    Deferred,
    Replayed,
}

#[derive(Debug, Clone)]
pub struct CreateOrderInput {
    pub access_token_hash: String,
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub address_type: String,
    pub zec_amount: Decimal,
    pub zec_rate_id: Uuid,
    pub required_confs: i32,
    pub expires_at: DateTime<Utc>,
    pub ip_hash: Option<String>,
    pub metadata: Value,
    pub variation_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrderRow {
    pub id: Uuid,
    pub status: String,
    pub access_token_hash: String,
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub deposit_address: String,
    pub address_type: String,
    pub zec_amount: Decimal,
    pub zec_rate_id: Uuid,
    pub txid: Option<String>,
    pub confirmations: i32,
    pub required_confs: i32,
    pub total_received: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub vtpass_request_id: Option<String>,
    pub delivery_token: Option<String>,
    pub ip_hash: Option<String>,
    pub metadata: Value,
    pub variation_code: Option<String>,
    pub provider: Option<String>,
    pub customer_name: Option<String>,
    pub user_id: Option<Uuid>,
}

impl OrderRow {
    pub fn status(&self) -> Result<OrderStatus> {
        match self.status.as_str() {
            "awaiting_payment" => Ok(OrderStatus::AwaitingPayment),
            "payment_detected" => Ok(OrderStatus::PaymentDetected),
            "payment_confirmed" => Ok(OrderStatus::PaymentConfirmed),
            "awaiting_claim" => Ok(OrderStatus::AwaitingClaim),
            "utility_dispatching" => Ok(OrderStatus::UtilityDispatching),
            "completed" => Ok(OrderStatus::Completed),
            "expired" => Ok(OrderStatus::Expired),
            "failed" => Ok(OrderStatus::Failed),
            "flagged_for_review" => Ok(OrderStatus::FlaggedForReview),
            "cancelled" => Ok(OrderStatus::Cancelled),
            other => Err(anyhow!("unknown order status: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressPoolDepth {
    pub address_type: String,
    pub unused_count: i64,
}

#[derive(Debug, Clone)]
pub struct PersistRateSnapshotInput {
    pub zec_ngn: Decimal,
    pub zec_usd: Decimal,
    pub usd_ngn: Decimal,
    pub usd_kes: Decimal,
    pub usd_ghs: Decimal,
    pub usd_zar: Decimal,
    pub usd_egp: Decimal,
    pub coingecko_zec_ngn: Option<Decimal>,
    pub binance_zec_usd: Option<Decimal>,
    pub kraken_zec_usd: Option<Decimal>,
    pub coinbase_zec_usd: Option<Decimal>,
    pub sources_used: Vec<String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

pub async fn begin_tx(pool: &PgPool) -> Result<Transaction<'_, Postgres>> {
    pool.begin().await.map_err(Into::into)
}

pub async fn claim_unused_deposit_address(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    address_type: &str,
    network: &str,
) -> Result<String> {
    let claimed = sqlx::query_scalar::<_, String>(
        "UPDATE deposit_addresses
         SET order_id = $1, used = true
         WHERE address = (
            SELECT address
            FROM deposit_addresses
            WHERE used = false AND address_type = $2 AND network = $3
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
         )
         RETURNING address",
    )
    .bind(order_id)
    .bind(address_type)
    .bind(network)
    .fetch_optional(tx.as_mut())
    .await?;

    claimed.ok_or_else(|| {
        anyhow!("no unused deposit address available for {address_type} on {network}")
    })
}

pub async fn count_unused_deposit_addresses(
    pool: &PgPool,
    address_type: &str,
    network: &str,
) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM deposit_addresses
         WHERE used = false AND address_type = $1 AND network = $2",
    )
    .bind(address_type)
    .bind(network)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn load_address_pool_depths(
    pool: &PgPool,
    network: &str,
) -> Result<Vec<AddressPoolDepth>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT address_type, COUNT(*) FILTER (WHERE used = false) AS unused_count
         FROM deposit_addresses
         WHERE network = $1
         GROUP BY address_type",
    )
    .bind(network)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(address_type, unused_count)| AddressPoolDepth {
            address_type,
            unused_count,
        })
        .collect())
}

pub async fn insert_deposit_addresses(
    pool: &PgPool,
    address_type: &str,
    network: &str,
    addresses: &[String],
) -> Result<u64> {
    if addresses.is_empty() {
        return Ok(0);
    }

    let mut inserted = 0_u64;
    for address in addresses {
        let affected = sqlx::query(
            "INSERT INTO deposit_addresses (address, address_type, network, used)
             VALUES ($1, $2, $3, false)
             ON CONFLICT (address) DO NOTHING",
        )
        .bind(address)
        .bind(address_type)
        .bind(network)
        .execute(pool)
        .await?
        .rows_affected();
        inserted += affected;
    }

    Ok(inserted)
}

pub async fn persist_rate_snapshot(
    pool: &PgPool,
    input: &PersistRateSnapshotInput,
) -> Result<Uuid> {
    let snapshot_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rate_snapshots (
            id,
            zec_ngn,
            zec_usd,
            usd_ngn,
            usd_kes,
            usd_ghs,
            usd_zar,
            usd_egp,
            coingecko_zec_ngn,
            binance_zec_usd,
            kraken_zec_usd,
            coinbase_zec_usd,
            sources_used,
            fetched_at
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            $13,
            $14
        )",
    )
    .bind(snapshot_id)
    .bind(input.zec_ngn)
    .bind(input.zec_usd)
    .bind(input.usd_ngn)
    .bind(input.usd_kes)
    .bind(input.usd_ghs)
    .bind(input.usd_zar)
    .bind(input.usd_egp)
    .bind(input.coingecko_zec_ngn)
    .bind(input.binance_zec_usd)
    .bind(input.kraken_zec_usd)
    .bind(input.coinbase_zec_usd)
    .bind(&input.sources_used)
    .bind(input.fetched_at)
    .execute(pool)
    .await?;

    Ok(snapshot_id)
}

pub async fn insert_order_with_claimed_address(
    tx: &mut Transaction<'_, Postgres>,
    input: &CreateOrderInput,
    network: &str,
) -> Result<(Uuid, String)> {
    let order_id = Uuid::new_v4();
    let deposit_address =
        claim_unused_deposit_address(tx, order_id, &input.address_type, network).await?;

    sqlx::query(
        "INSERT INTO orders (
            id,
            status,
            access_token_hash,
            utility_type,
            utility_slug,
            service_ref,
            amount_ngn,
            deposit_address,
            address_type,
            zec_amount,
            zec_rate_id,
            required_confs,
            expires_at,
            ip_hash,
            metadata,
            variation_code
         ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            $13,
            $14,
            $15,
            $16
         )",
    )
    .bind(order_id)
    .bind(OrderStatus::AwaitingPayment.as_db())
    .bind(&input.access_token_hash)
    .bind(&input.utility_type)
    .bind(&input.utility_slug)
    .bind(&input.service_ref)
    .bind(input.amount_ngn)
    .bind(&deposit_address)
    .bind(&input.address_type)
    .bind(input.zec_amount)
    .bind(input.zec_rate_id)
    .bind(input.required_confs)
    .bind(input.expires_at)
    .bind(&input.ip_hash)
    .bind(&input.metadata)
    .bind(&input.variation_code)
    .execute(tx.as_mut())
    .await?;

    Ok((order_id, deposit_address))
}

pub async fn apply_order_status_transition(
    tx: &mut Transaction<'_, Postgres>,
    transition: &OrderStatusTransitionRecord,
) -> Result<()> {
    let rows_affected = sqlx::query(
        "UPDATE orders
         SET status = $1
         WHERE id = $2
           AND status = $3",
    )
    .bind(transition.transition.to.as_db())
    .bind(transition.order_id)
    .bind(transition.transition.from.as_db())
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    if rows_affected != 1 {
        return Err(anyhow!(
            "status transition failed for order {}",
            transition.order_id
        ));
    }

    sqlx::query(
        "INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(transition.order_id)
    .bind(&transition.event)
    .bind(transition.transition.from.as_db())
    .bind(transition.transition.to.as_db())
    .bind(&transition.detail)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn get_order_by_id(pool: &PgPool, order_id: Uuid) -> Result<Option<OrderRow>> {
    let row = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    row.map(map_order_row).transpose()
}

pub async fn update_order_status_cas(
    pool: &PgPool,
    order_id: Uuid,
    from_status: &str,
    to_status: &str,
    event: &str,
    detail: Value,
) -> Result<bool> {
    let rows_affected = sqlx::query("UPDATE orders SET status = $1 WHERE id = $2 AND status = $3")
        .bind(to_status)
        .bind(order_id)
        .bind(from_status)
        .execute(pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(order_id)
    .bind(event)
    .bind(from_status)
    .bind(to_status)
    .bind(detail)
    .execute(pool)
    .await?;

    Ok(true)
}

pub async fn record_payment(
    pool: &PgPool,
    order_id: Uuid,
    txid: &str,
    address: &str,
    amount: Decimal,
    note_index: i32,
    confirmations: i32,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO payments (order_id, txid, address, amount_zec, note_index, confirmations)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (txid, address, amount_zec, note_index) DO UPDATE
         SET confirmations = EXCLUDED.confirmations,
             order_id = EXCLUDED.order_id,
             status = 'seen',
             updated_at = now()
         WHERE payments.confirmations IS DISTINCT FROM EXCLUDED.confirmations
            OR payments.order_id IS DISTINCT FROM EXCLUDED.order_id
            OR payments.status IS DISTINCT FROM 'seen'",
    )
    .bind(order_id)
    .bind(txid)
    .bind(address)
    .bind(amount)
    .bind(note_index)
    .bind(confirmations)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_missing_payments(pool: &PgPool, order_id: Uuid, txids: &[String]) -> Result<()> {
    sqlx::query(
        "UPDATE payments
         SET status = 'dropped', updated_at = now()
         WHERE order_id = $1 AND status = 'seen' AND txid <> ALL($2)",
    )
    .bind(order_id)
    .bind(txids)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_order_payment(
    pool: &PgPool,
    order_id: Uuid,
    confirmations: i32,
    total_received: Decimal,
    txid: Option<&str>,
) -> Result<()> {
    let mut query = String::from("UPDATE orders SET confirmations = $1, total_received = $2");
    if txid.is_some() {
        query.push_str(", txid = $4");
    }
    query.push_str(" WHERE id = $3");

    if txid.is_some() {
        sqlx::query(&query)
            .bind(confirmations)
            .bind(total_received)
            .bind(order_id)
            .bind(txid)
            .execute(pool)
            .await?;
    } else {
        sqlx::query(&query)
            .bind(confirmations)
            .bind(total_received)
            .bind(order_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn complete_order(
    pool: &PgPool,
    order_id: Uuid,
    delivery_token: Option<&str>,
    vtpass_request_id: Option<&str>,
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE orders
         SET status = 'completed', completed_at = now(), delivery_token = $2, vtpass_request_id = COALESCE($3, vtpass_request_id)
         WHERE id = $1 AND status = 'utility_dispatching'",
    )
    .bind(order_id)
    .bind(delivery_token)
    .bind(vtpass_request_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn fail_order(pool: &PgPool, order_id: Uuid) -> Result<bool> {
    let mut tx = begin_tx(pool).await?;

    let reference: Option<String> = sqlx::query_scalar(
        "SELECT vtpass_request_id FROM orders
         WHERE id = $1 AND status = 'utility_dispatching'
         FOR UPDATE",
    )
    .bind(order_id)
    .fetch_optional(tx.as_mut())
    .await?
    .flatten();

    let rows_affected = sqlx::query(
        "UPDATE orders SET status = 'failed', vtpass_request_id = NULL
         WHERE id = $1 AND status = 'utility_dispatching'",
    )
    .bind(order_id)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    if rows_affected > 0 {
        sqlx::query(
            "INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
             VALUES ($1, 'order_failed', 'utility_dispatching', 'failed', $2)",
        )
        .bind(order_id)
        .bind(serde_json::json!({ "reference": reference }))
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;
    Ok(rows_affected > 0)
}

pub async fn store_provider_reference(
    pool: &PgPool,
    order_id: Uuid,
    reference: &str,
) -> Result<()> {
    sqlx::query("UPDATE orders SET vtpass_request_id = $2 WHERE id = $1")
        .bind(order_id)
        .bind(reference)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn set_order_dispatching(
    pool: &PgPool,
    order_id: Uuid,
    provider: &str,
    vtpass_request_id: Option<&str>,
) -> Result<bool> {
    let mut tx = begin_tx(pool).await?;

    let rows_affected = sqlx::query(
        "UPDATE orders
         SET status = 'utility_dispatching', provider = $2, vtpass_request_id = COALESCE($3, vtpass_request_id)
         WHERE id = $1 AND status = 'payment_confirmed'",
    )
    .bind(order_id)
    .bind(provider)
    .bind(vtpass_request_id)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    if rows_affected > 0 {
        sqlx::query(
            "INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
             VALUES ($1, 'dispatch_started', 'payment_confirmed', 'utility_dispatching', $2)",
        )
        .bind(order_id)
        .bind(serde_json::json!({
            "provider": provider,
            "reference": vtpass_request_id,
        }))
        .execute(tx.as_mut())
        .await?;
    }

    tx.commit().await?;
    Ok(rows_affected > 0)
}

pub async fn mark_provider_accepted(pool: &PgPool, order_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE orders
         SET metadata = metadata || '{\"provider_accepted\": true}'::jsonb
         WHERE id = $1",
    )
    .bind(order_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn complete_flagged_school_fee_order(
    pool: &PgPool,
    order_id: Uuid,
    delivery_token: Option<&str>,
    provider_request_id: Option<&str>,
) -> Result<bool> {
    let rows_affected = sqlx::query(
        "UPDATE orders
         SET status = 'completed',
             delivery_token = COALESCE($2, delivery_token),
             vtpass_request_id = COALESCE($3, vtpass_request_id),
             completed_at = now()
         WHERE id = $1
           AND status = 'flagged_for_review'
           AND utility_type = 'school_fees'",
    )
    .bind(order_id)
    .bind(delivery_token)
    .bind(provider_request_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

pub async fn active_school_fee_order_exists(pool: &PgPool, service_ref: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM orders
            WHERE utility_type = 'school_fees'
              AND service_ref = $1
              AND status IN (
                  'awaiting_payment',
                  'payment_detected',
                  'payment_confirmed',
                  'utility_dispatching',
                  'flagged_for_review'
              )
        )",
    )
    .bind(service_ref)
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

pub async fn dispatch_started_at(pool: &PgPool, order_id: Uuid) -> Result<Option<DateTime<Utc>>> {
    let started_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT created_at FROM audit_log
         WHERE order_id = $1 AND event = 'dispatch_started'
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    Ok(started_at)
}

pub async fn list_active_orders(pool: &PgPool) -> Result<Vec<OrderRow>> {
    let rows = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders
         WHERE (status IN ('awaiting_payment', 'payment_detected', 'payment_confirmed', 'awaiting_claim', 'utility_dispatching')
                OR (status = 'flagged_for_review' AND utility_type = 'school_fees'))
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).collect()
}

pub async fn find_order_by_provider_reference(
    pool: &PgPool,
    reference: &str,
) -> Result<Option<OrderRow>> {
    let rows = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders
         WHERE (vtpass_request_id = $1 AND status = 'utility_dispatching')
            OR (vtpass_request_id = $1 AND status = 'flagged_for_review' AND utility_type = 'school_fees')",
    )
    .bind(reference)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).next().transpose()
}

pub async fn find_order_for_webhook(pool: &PgPool, reference: &str) -> Result<Option<OrderRow>> {
    let row = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders
         WHERE vtpass_request_id = $1
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?;

    row.map(map_order_row).transpose()
}

pub async fn record_webhook_and_transition(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
    event_key: &str,
    order_id: Uuid,
    transition: Option<&WebhookTransition>,
) -> Result<WebhookTransitionOutcome> {
    let inserted = sqlx::query(
        "INSERT INTO webhook_events (provider, event_key) VALUES ($1, $2)
         ON CONFLICT (provider, event_key) DO NOTHING",
    )
    .bind(provider)
    .bind(event_key)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    if inserted == 0 {
        return Ok(WebhookTransitionOutcome::Replayed);
    }

    let Some(transition) = transition else {
        return Ok(WebhookTransitionOutcome::Recorded);
    };

    let updated = sqlx::query(
        "UPDATE orders
         SET status = $3,
             delivery_token = COALESCE($4, delivery_token),
             vtpass_request_id = COALESCE($5, vtpass_request_id)
         WHERE id = $1 AND status = $2",
    )
    .bind(order_id)
    .bind(&transition.from_status)
    .bind(&transition.to_status)
    .bind(&transition.delivery_token)
    .bind(&transition.provider_reference)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    if updated == 0 {
        sqlx::query("DELETE FROM webhook_events WHERE provider = $1 AND event_key = $2")
            .bind(provider)
            .bind(event_key)
            .execute(tx.as_mut())
            .await?;
        return Ok(WebhookTransitionOutcome::Deferred);
    }

    sqlx::query(
        "INSERT INTO audit_log (order_id, event, old_status, new_status, detail)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(order_id)
    .bind(&transition.event)
    .bind(&transition.from_status)
    .bind(&transition.to_status)
    .bind(&transition.detail)
    .execute(tx.as_mut())
    .await?;

    Ok(WebhookTransitionOutcome::Applied)
}

#[derive(Debug, Clone)]
pub struct GiftRow {
    pub id: Uuid,
    pub code_hash: String,
    pub sender_user_id: Uuid,
    pub order_id: Option<Uuid>,
    pub utility_type: String,
    pub utility_slug: String,
    pub amount_ngn: i64,
    pub claim_window_hours: i32,
    pub message: Option<String>,
    pub sender_name: Option<String>,
    pub hide_sender_name: bool,
    pub fallback_service_ref: Option<String>,
    pub status: String,
    pub claimed_service_ref: Option<String>,
    pub claimed_customer_name: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub claim_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewGift {
    pub code_hash: String,
    pub sender_user_id: Uuid,
    pub order_id: Uuid,
    pub utility_type: String,
    pub utility_slug: String,
    pub amount_ngn: i64,
    pub claim_window_hours: i32,
    pub message: Option<String>,
    pub sender_name: Option<String>,
    pub hide_sender_name: bool,
    pub fallback_service_ref: Option<String>,
    pub claim_expires_at: DateTime<Utc>,
}

impl GiftRow {
    pub fn status(&self) -> Option<crate::domain::gift::GiftStatus> {
        crate::domain::gift::GiftStatus::from_db(&self.status)
    }
}

fn map_gift_row(row: sqlx::postgres::PgRow) -> Result<GiftRow> {
    Ok(GiftRow {
        id: row.try_get("id")?,
        code_hash: row.try_get("code_hash")?,
        sender_user_id: row.try_get("sender_user_id")?,
        order_id: row.try_get("order_id")?,
        utility_type: row.try_get("utility_type")?,
        utility_slug: row.try_get("utility_slug")?,
        amount_ngn: row.try_get("amount_ngn")?,
        claim_window_hours: row.try_get("claim_window_hours")?,
        message: row.try_get("message")?,
        sender_name: row.try_get("sender_name")?,
        hide_sender_name: row.try_get("hide_sender_name")?,
        fallback_service_ref: row.try_get("fallback_service_ref")?,
        status: row.try_get("status")?,
        claimed_service_ref: row.try_get("claimed_service_ref")?,
        claimed_customer_name: row.try_get("claimed_customer_name")?,
        claimed_at: row.try_get("claimed_at")?,
        claim_expires_at: row.try_get("claim_expires_at")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn lock_user_and_sum_open_gifts(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<i64> {
    let exists = sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(tx.as_mut())
        .await?;

    if exists.is_none() {
        return Err(anyhow!("user not found"));
    }

    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_ngn), 0)::bigint
         FROM gift_invites
         WHERE sender_user_id = $1 AND status = 'open'",
    )
    .bind(user_id)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(total)
}

pub async fn insert_gift_tx(tx: &mut Transaction<'_, Postgres>, gift: &NewGift) -> Result<Uuid> {
    let gift_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gift_invites (
            id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
            amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
            fallback_service_ref, claim_expires_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(gift_id)
    .bind(&gift.code_hash)
    .bind(gift.sender_user_id)
    .bind(gift.order_id)
    .bind(&gift.utility_type)
    .bind(&gift.utility_slug)
    .bind(gift.amount_ngn)
    .bind(gift.claim_window_hours)
    .bind(&gift.message)
    .bind(&gift.sender_name)
    .bind(gift.hide_sender_name)
    .bind(&gift.fallback_service_ref)
    .bind(gift.claim_expires_at)
    .execute(tx.as_mut())
    .await?;

    Ok(gift_id)
}

pub async fn find_gift_by_code_hash(pool: &PgPool, code_hash: &str) -> Result<Option<GiftRow>> {
    let row = sqlx::query(
        "SELECT id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
                amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
                fallback_service_ref, status, claimed_service_ref, claimed_customer_name,
                claimed_at, claim_expires_at, created_at
         FROM gift_invites WHERE code_hash = $1",
    )
    .bind(code_hash)
    .fetch_optional(pool)
    .await?;

    row.map(map_gift_row).transpose()
}

pub async fn find_gift_by_id(pool: &PgPool, gift_id: Uuid) -> Result<Option<GiftRow>> {
    let row = sqlx::query(
        "SELECT id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
                amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
                fallback_service_ref, status, claimed_service_ref, claimed_customer_name,
                claimed_at, claim_expires_at, created_at
         FROM gift_invites WHERE id = $1",
    )
    .bind(gift_id)
    .fetch_optional(pool)
    .await?;

    row.map(map_gift_row).transpose()
}

pub async fn find_gift_by_order_id(pool: &PgPool, order_id: Uuid) -> Result<Option<GiftRow>> {
    let row = sqlx::query(
        "SELECT id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
                amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
                fallback_service_ref, status, claimed_service_ref, claimed_customer_name,
                claimed_at, claim_expires_at, created_at
         FROM gift_invites WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    row.map(map_gift_row).transpose()
}

pub async fn list_gifts_by_sender(pool: &PgPool, user_id: Uuid) -> Result<Vec<GiftRow>> {
    let rows = sqlx::query(
        "SELECT id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
                amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
                fallback_service_ref, status, claimed_service_ref, claimed_customer_name,
                claimed_at, claim_expires_at, created_at
         FROM gift_invites WHERE sender_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_gift_row).collect()
}

pub async fn claim_gift_tx(
    tx: &mut Transaction<'_, Postgres>,
    gift_id: Uuid,
    service_ref: &str,
    customer_name: Option<&str>,
) -> Result<bool> {
    let rows_affected = sqlx::query(
        "UPDATE gift_invites
         SET status = 'claimed', claimed_service_ref = $2, claimed_customer_name = $3, claimed_at = now()
         WHERE id = $1 AND status = 'open'",
    )
    .bind(gift_id)
    .bind(service_ref)
    .bind(customer_name)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    Ok(rows_affected == 1)
}

pub async fn update_order_service_ref_tx(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    service_ref: &str,
    customer_name: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE orders SET service_ref = $2, customer_name = $3 WHERE id = $1")
        .bind(order_id)
        .bind(service_ref)
        .bind(customer_name)
        .execute(tx.as_mut())
        .await?;

    Ok(())
}

pub async fn update_order_service_ref(
    pool: &PgPool,
    order_id: Uuid,
    service_ref: &str,
    customer_name: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE orders SET service_ref = $2, customer_name = $3 WHERE id = $1")
        .bind(order_id)
        .bind(service_ref)
        .bind(customer_name)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn set_order_access_token_hash(
    pool: &PgPool,
    order_id: Uuid,
    access_token_hash: &str,
) -> Result<()> {
    sqlx::query("UPDATE orders SET access_token_hash = $2 WHERE id = $1")
        .bind(order_id)
        .bind(access_token_hash)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn lock_order_status_tx(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
) -> Result<Option<String>> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM orders WHERE id = $1 FOR UPDATE")
            .bind(order_id)
            .fetch_optional(tx.as_mut())
            .await?;

    Ok(status)
}

pub async fn set_gift_code_hash(pool: &PgPool, gift_id: Uuid, code_hash: &str) -> Result<bool> {
    let rows_affected =
        sqlx::query("UPDATE gift_invites SET code_hash = $2 WHERE id = $1 AND status = 'open'")
            .bind(gift_id)
            .bind(code_hash)
            .execute(pool)
            .await?
            .rows_affected();

    Ok(rows_affected == 1)
}

pub async fn cancel_gift_tx(tx: &mut Transaction<'_, Postgres>, gift_id: Uuid) -> Result<bool> {
    let rows_affected = sqlx::query(
        "UPDATE gift_invites SET status = 'cancelled' WHERE id = $1 AND status = 'open'",
    )
    .bind(gift_id)
    .execute(tx.as_mut())
    .await?
    .rows_affected();

    Ok(rows_affected == 1)
}

pub async fn cancel_gift_by_order_id(pool: &PgPool, order_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE gift_invites SET status = 'cancelled' WHERE order_id = $1 AND status = 'open'",
    )
    .bind(order_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_order_statuses_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String)>> {
    let rows = sqlx::query("SELECT id, status FROM orders WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    rows.into_iter()
        .map(|row| Ok((row.try_get("id")?, row.try_get("status")?)))
        .collect()
}

pub async fn set_gift_claim_expiry(
    pool: &PgPool,
    gift_id: Uuid,
    claim_expires_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query("UPDATE gift_invites SET claim_expires_at = $2 WHERE id = $1 AND status = 'open'")
        .bind(gift_id)
        .bind(claim_expires_at)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn expire_gift_cas(pool: &PgPool, gift_id: Uuid) -> Result<bool> {
    let rows_affected =
        sqlx::query("UPDATE gift_invites SET status = 'expired' WHERE id = $1 AND status = 'open'")
            .bind(gift_id)
            .execute(pool)
            .await?
            .rows_affected();

    Ok(rows_affected == 1)
}

pub async fn list_expired_open_gifts(pool: &PgPool, now: DateTime<Utc>) -> Result<Vec<GiftRow>> {
    let rows = sqlx::query(
        "SELECT id, code_hash, sender_user_id, order_id, utility_type, utility_slug,
                amount_ngn, claim_window_hours, message, sender_name, hide_sender_name,
                fallback_service_ref, status, claimed_service_ref, claimed_customer_name,
                claimed_at, claim_expires_at, created_at
         FROM gift_invites
         WHERE status = 'open' AND claim_expires_at IS NOT NULL AND claim_expires_at <= $1",
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_gift_row).collect()
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub email_verified: bool,
    pub preferred_currency: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub replaced_by: Option<Uuid>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct ClaimedRefreshToken {
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EmailTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub token_type: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    display_name: Option<&str>,
    password_hash: &str,
    preferred_currency: &str,
) -> Result<UserRow> {
    let row = sqlx::query(
        "INSERT INTO users (email, display_name, password_hash, preferred_currency)
         VALUES (LOWER($1), $2, $3, $4)
         RETURNING id, email, display_name, password_hash, email_verified, preferred_currency, created_at, updated_at, deleted_at",
    )
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(preferred_currency)
    .fetch_one(pool)
    .await?;

    map_user_row(row)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, password_hash, email_verified, preferred_currency, created_at, updated_at, deleted_at
         FROM users WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    row.map(map_user_row).transpose()
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, password_hash, email_verified, preferred_currency, created_at, updated_at, deleted_at
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(map_user_row).transpose()
}

pub async fn find_user_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, password_hash, email_verified, preferred_currency, created_at, updated_at, deleted_at
         FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;

    row.map(map_user_row).transpose()
}

pub async fn update_user_password(pool: &PgPool, user_id: Uuid, password_hash: &str) -> Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_user_display_name(
    pool: &PgPool,
    user_id: Uuid,
    display_name: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE users SET display_name = COALESCE($2, display_name), updated_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(display_name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_user_preferred_currency(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE users SET preferred_currency = $2, updated_at = now() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(code)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn soft_delete_user(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE users SET deleted_at = now(), updated_at = now(), email = email || '.' || gen_random_uuid()::text WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn verify_user_email(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE users SET email_verified = true, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_refresh_token(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: chrono::DateTime<chrono::Utc>,
    user_agent: Option<&str>,
    ip_hash: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, family_id, expires_at, user_agent, ip_hash)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(family_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_refresh_token_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: chrono::DateTime<chrono::Utc>,
    user_agent: Option<&str>,
    ip_hash: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, family_id, expires_at, user_agent, ip_hash)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(family_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_hash)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn refresh_token_owner_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<Uuid>> {
    let owner = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM refresh_tokens WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(owner)
}

pub async fn lock_user_sessions_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn claim_refresh_token_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<ClaimedRefreshToken>> {
    let row = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now()
         WHERE id = $1 AND revoked_at IS NULL
         RETURNING user_id, family_id, expires_at",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| {
        Ok(ClaimedRefreshToken {
            user_id: row.try_get("user_id")?,
            family_id: row.try_get("family_id")?,
            expires_at: row.try_get("expires_at")?,
        })
    })
    .transpose()
}

pub async fn link_refresh_token_replacement_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    replacement: Uuid,
) -> Result<()> {
    sqlx::query("UPDATE refresh_tokens SET replaced_by = $2 WHERE id = $1")
        .bind(id)
        .bind(replacement)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

pub async fn find_live_successor(pool: &PgPool, id: Uuid) -> Result<Option<Uuid>> {
    let row = sqlx::query(
        "WITH RECURSIVE chain AS (
             SELECT id, replaced_by, revoked_at, expires_at, 1 AS depth
             FROM refresh_tokens WHERE id = $1
             UNION ALL
             SELECT t.id, t.replaced_by, t.revoked_at, t.expires_at, c.depth + 1
             FROM refresh_tokens t JOIN chain c ON t.id = c.replaced_by
             WHERE c.depth < 8
         )
         SELECT id FROM chain
         WHERE revoked_at IS NULL AND expires_at > now()
         ORDER BY depth DESC
         LIMIT 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(|row| row.try_get::<Uuid, _>("id").map_err(Into::into))
        .transpose()
}

pub async fn find_refresh_token(pool: &PgPool, id: Uuid) -> Result<Option<RefreshTokenRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, family_id, replaced_by, expires_at, created_at, revoked_at
         FROM refresh_tokens WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(RefreshTokenRow {
            id: r.try_get("id")?,
            user_id: r.try_get("user_id")?,
            family_id: r.try_get("family_id")?,
            replaced_by: r.try_get("replaced_by")?,
            expires_at: r.try_get("expires_at")?,
            created_at: r.try_get("created_at")?,
            revoked_at: r.try_get("revoked_at")?,
        })
    })
    .transpose()
}

pub async fn revoke_refresh_token(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke_refresh_token_family(pool: &PgPool, token_id: Uuid) -> Result<()> {
    let mut tx = begin_tx(pool).await?;
    let owner = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(user_id) = owner {
        lock_user_sessions_tx(&mut tx, user_id).await?;
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now()
             WHERE revoked_at IS NULL
               AND family_id = (SELECT family_id FROM refresh_tokens WHERE id = $1)",
        )
        .bind(token_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn revoke_all_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<()> {
    let mut tx = begin_tx(pool).await?;
    lock_user_sessions_tx(&mut tx, user_id).await?;
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn create_email_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    token_type: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO email_verification_tokens (user_id, token_hash, token_type, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(token_type)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_email_token_by_hash(
    pool: &PgPool,
    token_hash: &str,
    token_type: &str,
) -> Result<Option<EmailTokenRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, token_hash, token_type, expires_at, used_at
         FROM email_verification_tokens
         WHERE token_hash = $1 AND token_type = $2 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(token_hash)
    .bind(token_type)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(EmailTokenRow {
            id: r.try_get("id")?,
            user_id: r.try_get("user_id")?,
            token_hash: r.try_get("token_hash")?,
            token_type: r.try_get("token_type")?,
            expires_at: r.try_get("expires_at")?,
            used_at: r.try_get("used_at")?,
        })
    })
    .transpose()
}

pub async fn mark_email_token_used(pool: &PgPool, id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE email_verification_tokens SET used_at = now() WHERE id = $1 AND used_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn invalidate_email_tokens(pool: &PgPool, user_id: Uuid, token_type: &str) -> Result<()> {
    sqlx::query(
        "UPDATE email_verification_tokens SET used_at = now()
         WHERE user_id = $1 AND token_type = $2 AND used_at IS NULL",
    )
    .bind(user_id)
    .bind(token_type)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn login_lockout_until(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    let locked_until = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT locked_until FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(locked_until)
}

pub async fn record_failed_login(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE users
         SET failed_login_attempts = failed_login_attempts + 1,
             locked_until = CASE
                 WHEN failed_login_attempts + 1 >= 5 THEN now() + interval '15 minutes'
                 ELSE locked_until
             END
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn clear_failed_logins(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE users SET failed_login_attempts = 0, locked_until = NULL
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_orders_by_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<OrderRow>> {
    let rows = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).collect()
}

pub async fn set_order_user_id(pool: &PgPool, order_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE orders SET user_id = $2 WHERE id = $1 AND user_id IS NULL")
        .bind(order_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn set_order_user_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    user_id: Uuid,
) -> Result<()> {
    sqlx::query("UPDATE orders SET user_id = $2 WHERE id = $1 AND user_id IS NULL")
        .bind(order_id)
        .bind(user_id)
        .execute(tx.as_mut())
        .await?;

    Ok(())
}

fn map_user_row(row: sqlx::postgres::PgRow) -> Result<UserRow> {
    Ok(UserRow {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        display_name: row.try_get("display_name")?,
        password_hash: row.try_get("password_hash")?,
        email_verified: row.try_get("email_verified")?,
        preferred_currency: row.try_get("preferred_currency")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        deleted_at: row.try_get("deleted_at")?,
    })
}

fn map_order_row(row: sqlx::postgres::PgRow) -> Result<OrderRow> {
    Ok(OrderRow {
        id: row.try_get("id")?,
        status: row.try_get("status")?,
        access_token_hash: row.try_get("access_token_hash")?,
        utility_type: row.try_get("utility_type")?,
        utility_slug: row.try_get("utility_slug")?,
        service_ref: row.try_get("service_ref")?,
        amount_ngn: row.try_get("amount_ngn")?,
        deposit_address: row.try_get("deposit_address")?,
        address_type: row.try_get("address_type")?,
        zec_amount: row.try_get("zec_amount")?,
        zec_rate_id: row.try_get("zec_rate_id")?,
        txid: row.try_get("txid")?,
        confirmations: row.try_get("confirmations")?,
        required_confs: row.try_get("required_confs")?,
        total_received: row.try_get("total_received")?,
        created_at: row.try_get("created_at")?,
        expires_at: row.try_get("expires_at")?,
        confirmed_at: row.try_get("confirmed_at")?,
        completed_at: row.try_get("completed_at")?,
        vtpass_request_id: row.try_get("vtpass_request_id")?,
        delivery_token: row.try_get("delivery_token")?,
        ip_hash: row.try_get("ip_hash")?,
        metadata: row.try_get("metadata")?,
        variation_code: row.try_get("variation_code")?,
        provider: row.try_get("provider")?,
        customer_name: row.try_get("customer_name")?,
        user_id: row.try_get("user_id")?,
    })
}

#[derive(Debug, Clone)]
pub struct WaitlistEntryRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub referral_code: String,
    pub referred_by: Option<String>,
    pub ip_hash: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub utm_content: Option<String>,
    pub utm_term: Option<String>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_waitlist_entry(
    pool: &PgPool,
    email: &str,
    display_name: Option<&str>,
    referral_code: &str,
    referred_by: Option<&str>,
    ip_hash: Option<&str>,
    utm_source: Option<&str>,
    utm_medium: Option<&str>,
    utm_campaign: Option<&str>,
    utm_content: Option<&str>,
    utm_term: Option<&str>,
) -> Result<WaitlistEntryRow> {
    let row = sqlx::query(
        "INSERT INTO waitlist_entries (email, display_name, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term)
         VALUES (LOWER($1), $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id, email, display_name, email_verified, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term, verified_at, created_at",
    )
    .bind(email)
    .bind(display_name)
    .bind(referral_code)
    .bind(referred_by)
    .bind(ip_hash)
    .bind(utm_source)
    .bind(utm_medium)
    .bind(utm_campaign)
    .bind(utm_content)
    .bind(utm_term)
    .fetch_one(pool)
    .await?;

    Ok(WaitlistEntryRow {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        display_name: row.try_get("display_name")?,
        email_verified: row.try_get("email_verified")?,
        referral_code: row.try_get("referral_code")?,
        referred_by: row.try_get("referred_by")?,
        ip_hash: row.try_get("ip_hash")?,
        utm_source: row.try_get("utm_source")?,
        utm_medium: row.try_get("utm_medium")?,
        utm_campaign: row.try_get("utm_campaign")?,
        utm_content: row.try_get("utm_content")?,
        utm_term: row.try_get("utm_term")?,
        verified_at: row.try_get("verified_at")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn find_waitlist_entry_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<WaitlistEntryRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, email_verified, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term, verified_at, created_at
         FROM waitlist_entries WHERE LOWER(email) = LOWER($1)",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(WaitlistEntryRow {
            id: r.try_get("id")?,
            email: r.try_get("email")?,
            display_name: r.try_get("display_name")?,
            email_verified: r.try_get("email_verified")?,
            referral_code: r.try_get("referral_code")?,
            referred_by: r.try_get("referred_by")?,
            ip_hash: r.try_get("ip_hash")?,
            utm_source: r.try_get("utm_source")?,
            utm_medium: r.try_get("utm_medium")?,
            utm_campaign: r.try_get("utm_campaign")?,
            utm_content: r.try_get("utm_content")?,
            utm_term: r.try_get("utm_term")?,
            verified_at: r.try_get("verified_at")?,
            created_at: r.try_get("created_at")?,
        })
    })
    .transpose()
}

pub async fn find_waitlist_entry_by_referral_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<WaitlistEntryRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, email_verified, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term, verified_at, created_at
         FROM waitlist_entries WHERE referral_code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(WaitlistEntryRow {
            id: r.try_get("id")?,
            email: r.try_get("email")?,
            display_name: r.try_get("display_name")?,
            email_verified: r.try_get("email_verified")?,
            referral_code: r.try_get("referral_code")?,
            referred_by: r.try_get("referred_by")?,
            ip_hash: r.try_get("ip_hash")?,
            utm_source: r.try_get("utm_source")?,
            utm_medium: r.try_get("utm_medium")?,
            utm_campaign: r.try_get("utm_campaign")?,
            utm_content: r.try_get("utm_content")?,
            utm_term: r.try_get("utm_term")?,
            verified_at: r.try_get("verified_at")?,
            created_at: r.try_get("created_at")?,
        })
    })
    .transpose()
}

pub async fn verify_waitlist_email(pool: &PgPool, entry_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE waitlist_entries SET email_verified = true, verified_at = now() WHERE id = $1",
    )
    .bind(entry_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_waitlist_position(pool: &PgPool, entry_id: Uuid) -> Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS pos FROM waitlist_entries WHERE created_at <= (SELECT created_at FROM waitlist_entries WHERE id = $1)",
    )
    .bind(entry_id)
    .fetch_one(pool)
    .await?;
    Ok(row.try_get("pos")?)
}

pub async fn count_waitlist_entries(pool: &PgPool) -> Result<(i64, i64)> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE email_verified) AS verified FROM waitlist_entries",
    )
    .fetch_one(pool)
    .await?;
    Ok((row.try_get("total")?, row.try_get("verified")?))
}

pub async fn create_waitlist_verify_token(
    pool: &PgPool,
    entry_id: Uuid,
    token_hash: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO waitlist_verify_tokens (entry_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(entry_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub struct WaitlistVerifyTokenRow {
    pub id: Uuid,
    pub entry_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn find_waitlist_verify_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<WaitlistVerifyTokenRow>> {
    let row = sqlx::query(
        "SELECT id, entry_id, expires_at, used_at FROM waitlist_verify_tokens WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    row.map(|r| {
        Ok(WaitlistVerifyTokenRow {
            id: r.try_get("id")?,
            entry_id: r.try_get("entry_id")?,
            expires_at: r.try_get("expires_at")?,
            used_at: r.try_get("used_at")?,
        })
    })
    .transpose()
}

pub async fn mark_waitlist_verify_token_used(pool: &PgPool, token_id: Uuid) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE waitlist_verify_tokens SET used_at = now() WHERE id = $1 AND used_at IS NULL",
    )
    .bind(token_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn invalidate_waitlist_verify_tokens(pool: &PgPool, entry_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE waitlist_verify_tokens SET used_at = now()
         WHERE entry_id = $1 AND used_at IS NULL",
    )
    .bind(entry_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn referral_code_exists(pool: &PgPool, code: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM waitlist_entries WHERE referral_code = $1) AS exists",
    )
    .bind(code)
    .fetch_one(pool)
    .await?;
    Ok(row.try_get("exists")?)
}

#[derive(Debug, Clone)]
pub struct NotificationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub order_id: Option<Uuid>,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub detail: Value,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_notification(
    pool: &PgPool,
    user_id: Uuid,
    order_id: Option<Uuid>,
    notification_type: &str,
    title: &str,
    body: &str,
    detail: Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO notifications (user_id, order_id, type, title, body, detail)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(order_id)
    .bind(notification_type)
    .bind(title)
    .bind(body)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<NotificationRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, order_id, type, title, body, detail, read_at, created_at
         FROM notifications
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_notification_row).collect()
}

pub async fn count_unread_notifications(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn mark_notification_read(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool> {
    let affected = sqlx::query(
        "UPDATE notifications SET read_at = now() WHERE id = $1 AND user_id = $2 AND read_at IS NULL",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected > 0)
}

pub async fn mark_all_notifications_read(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn map_notification_row(row: sqlx::postgres::PgRow) -> Result<NotificationRow> {
    Ok(NotificationRow {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        order_id: row.try_get("order_id")?,
        notification_type: row.try_get("type")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        detail: row.try_get("detail")?,
        read_at: row.try_get("read_at")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn list_catalogue(
    pool: &PgPool,
    country: Option<&str>,
) -> Result<Vec<crate::domain::catalogue::CatalogueCountry>> {
    let rows = sqlx::query(
        "SELECT c.iso2, c.name AS country_name, c.currency,
                p.id, p.utility_category, p.slug, p.name AS product_name, p.descriptor,
                p.icon_key, p.service_ref_label, p.service_ref_placeholder,
                p.service_ref_pattern, p.quick_amounts, p.amount_kind, p.amount_min,
                p.amount_max, p.amount_increment, p.destination_currency,
                p.fulfilment_kind, p.operator_id, p.operator_name, p.required_fields,
                p.field_labels
         FROM countries c
         JOIN provider_products p ON p.country_iso2 = c.iso2 AND p.enabled
         WHERE c.enabled AND ($1::text IS NULL OR c.iso2::text = $1)
         ORDER BY c.iso2, p.utility_category, p.name",
    )
    .bind(country)
    .fetch_all(pool)
    .await?;

    let product_ids: Vec<Uuid> = rows
        .iter()
        .map(|row| row.try_get::<Uuid, _>("id"))
        .collect::<std::result::Result<_, _>>()?;

    let mut variations: std::collections::HashMap<
        Uuid,
        Vec<crate::domain::catalogue::CatalogueVariation>,
    > = std::collections::HashMap::new();
    if !product_ids.is_empty() {
        let variation_rows = sqlx::query(
            "SELECT product_id, variation_code, name, amount
             FROM provider_product_variations
             WHERE enabled AND product_id = ANY($1)
             ORDER BY amount, name",
        )
        .bind(&product_ids)
        .fetch_all(pool)
        .await?;
        for row in variation_rows {
            variations
                .entry(row.try_get("product_id")?)
                .or_default()
                .push(crate::domain::catalogue::CatalogueVariation {
                    variation_code: row.try_get("variation_code")?,
                    name: row.try_get("name")?,
                    amount: catalogue_amount(row.try_get("amount")?),
                });
        }
    }

    let mut country_order: Vec<String> = Vec::new();
    let mut country_meta: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    let mut country_products: std::collections::HashMap<
        String,
        Vec<crate::domain::catalogue::CatalogueProduct>,
    > = std::collections::HashMap::new();

    for row in rows {
        let iso2: String = row.try_get("iso2")?;
        if !country_meta.contains_key(&iso2) {
            country_order.push(iso2.clone());
            country_meta.insert(
                iso2.clone(),
                (row.try_get("country_name")?, row.try_get("currency")?),
            );
        }
        let product_id: Uuid = row.try_get("id")?;
        let product = crate::domain::catalogue::CatalogueProduct {
            id: product_id,
            category: row.try_get("utility_category")?,
            slug: row.try_get("slug")?,
            name: row.try_get("product_name")?,
            descriptor: row.try_get("descriptor")?,
            icon_key: row.try_get("icon_key")?,
            service_ref_label: row.try_get("service_ref_label")?,
            service_ref_placeholder: row.try_get("service_ref_placeholder")?,
            service_ref_pattern: row.try_get("service_ref_pattern")?,
            quick_amounts: row.try_get("quick_amounts")?,
            amount_kind: row.try_get("amount_kind")?,
            amount_min: row
                .try_get::<Option<Decimal>, _>("amount_min")?
                .map(catalogue_amount),
            amount_max: row
                .try_get::<Option<Decimal>, _>("amount_max")?
                .map(catalogue_amount),
            amount_increment: row
                .try_get::<Option<Decimal>, _>("amount_increment")?
                .map(catalogue_amount),
            currency: row.try_get("destination_currency")?,
            fulfilment_kind: row.try_get("fulfilment_kind")?,
            operator_id: row.try_get("operator_id")?,
            operator_name: row.try_get("operator_name")?,
            required_fields: row.try_get("required_fields")?,
            field_labels: row.try_get("field_labels")?,
            variations: variations.remove(&product_id).unwrap_or_default(),
        };
        country_products.entry(iso2).or_default().push(product);
    }

    Ok(country_order
        .into_iter()
        .filter_map(|iso2| {
            let (name, currency) = country_meta.remove(&iso2)?;
            let products = country_products.remove(&iso2)?;
            Some(crate::domain::catalogue::CatalogueCountry {
                iso2,
                name,
                currency,
                categories: crate::domain::catalogue::group_by_category(products),
            })
        })
        .collect())
}

fn catalogue_amount(value: Decimal) -> String {
    value.normalize().to_string()
}
