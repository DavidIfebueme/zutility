use crate::domain::order::{OrderStatus, OrderStatusTransition};
use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
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
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub ip_hash: Option<String>,
    pub metadata: Value,
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
            $10
        )",
    )
    .bind(snapshot_id)
    .bind(input.zec_ngn)
    .bind(input.zec_usd)
    .bind(input.usd_ngn)
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
) -> Result<Uuid> {
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
            metadata
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
            $15
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
    .execute(tx.as_mut())
    .await?;

    Ok(order_id)
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
