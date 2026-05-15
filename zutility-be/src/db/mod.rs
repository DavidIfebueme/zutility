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
}

impl OrderRow {
    pub fn status(&self) -> Result<OrderStatus> {
        match self.status.as_str() {
            "awaiting_payment" => Ok(OrderStatus::AwaitingPayment),
            "payment_detected" => Ok(OrderStatus::PaymentDetected),
            "payment_confirmed" => Ok(OrderStatus::PaymentConfirmed),
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
) -> Result<String> {
    let claimed = sqlx::query_scalar::<_, String>(
        "UPDATE deposit_addresses
         SET order_id = $1, used = true
         WHERE address = (
            SELECT address
            FROM deposit_addresses
            WHERE used = false AND address_type = $2
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
         )
         RETURNING address",
    )
    .bind(order_id)
    .bind(address_type)
    .fetch_optional(tx.as_mut())
    .await?;

    claimed.ok_or_else(|| anyhow!("no unused deposit address available for {address_type}"))
}

pub async fn count_unused_deposit_addresses(pool: &PgPool, address_type: &str) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM deposit_addresses
         WHERE used = false AND address_type = $1",
    )
    .bind(address_type)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn load_address_pool_depths(pool: &PgPool) -> Result<Vec<AddressPoolDepth>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT address_type, COUNT(*) FILTER (WHERE used = false) AS unused_count
         FROM deposit_addresses
         GROUP BY address_type",
    )
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
    addresses: &[String],
) -> Result<u64> {
    if addresses.is_empty() {
        return Ok(0);
    }

    let mut inserted = 0_u64;
    for address in addresses {
        let affected = sqlx::query(
            "INSERT INTO deposit_addresses (address, address_type, used)
             VALUES ($1, $2, false)
             ON CONFLICT (address) DO NOTHING",
        )
        .bind(address)
        .bind(address_type)
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
) -> Result<(Uuid, String)> {
    let order_id = Uuid::new_v4();
    let deposit_address = claim_unused_deposit_address(tx, order_id, &input.address_type).await?;

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
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name
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
    let rows_affected = sqlx::query(
        "UPDATE orders SET status = $1 WHERE id = $2 AND status = $3",
    )
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

pub async fn update_order_payment(
    pool: &PgPool,
    order_id: Uuid,
    confirmations: i32,
    total_received: Decimal,
    txid: Option<&str>,
) -> Result<()> {
    let mut query = String::from(
        "UPDATE orders SET confirmations = $1, total_received = $2",
    );
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
) -> Result<()> {
    sqlx::query(
        "UPDATE orders
         SET status = 'completed', completed_at = now(), delivery_token = $2, vtpass_request_id = COALESCE($3, vtpass_request_id)
         WHERE id = $1 AND status = 'utility_dispatching'",
    )
    .bind(order_id)
    .bind(delivery_token)
    .bind(vtpass_request_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fail_order(pool: &PgPool, order_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE orders SET status = 'failed' WHERE id = $1 AND status = 'utility_dispatching'",
    )
    .bind(order_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn store_provider_reference(pool: &PgPool, order_id: Uuid, reference: &str) -> Result<()> {
    sqlx::query(
        "UPDATE orders SET vtpass_request_id = $2 WHERE id = $1 AND vtpass_request_id IS NULL",
    )
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
    let rows_affected = sqlx::query(
        "UPDATE orders
         SET status = 'utility_dispatching', provider = $2, vtpass_request_id = COALESCE($3, vtpass_request_id)
         WHERE id = $1 AND status = 'payment_confirmed'",
    )
    .bind(order_id)
    .bind(provider)
    .bind(vtpass_request_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

pub async fn list_active_orders(pool: &PgPool) -> Result<Vec<OrderRow>> {
    let rows = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name
         FROM orders
         WHERE status IN ('awaiting_payment', 'payment_detected', 'payment_confirmed', 'utility_dispatching')
         ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).collect()
}

pub async fn find_order_by_provider_reference(pool: &PgPool, reference: &str) -> Result<Option<OrderRow>> {
    let rows = sqlx::query(
        "SELECT id, status, access_token_hash, utility_type, utility_slug, service_ref,
                amount_ngn, deposit_address, address_type, zec_amount, zec_rate_id,
                txid, confirmations, required_confs, total_received, created_at,
                expires_at, confirmed_at, completed_at, vtpass_request_id,
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name
         FROM orders
         WHERE vtpass_request_id = $1 AND status = 'utility_dispatching'",
    )
    .bind(reference)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).next().transpose()
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
    })
}
