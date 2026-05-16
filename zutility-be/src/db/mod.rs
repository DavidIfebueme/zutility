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
    pub user_id: Option<Uuid>,
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
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
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
                delivery_token, ip_hash, metadata, variation_code, provider, customer_name, user_id
         FROM orders
         WHERE vtpass_request_id = $1 AND status = 'utility_dispatching'",
    )
    .bind(reference)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(map_order_row).next().transpose()
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub email_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
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
) -> Result<UserRow> {
    let row = sqlx::query(
        "INSERT INTO users (email, display_name, password_hash)
         VALUES (LOWER($1), $2, $3)
         RETURNING id, email, display_name, password_hash, email_verified, created_at, updated_at",
    )
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    map_user_row(row)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, password_hash, email_verified, created_at, updated_at
         FROM users WHERE LOWER(email) = LOWER($1)",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    row.map(map_user_row).transpose()
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, password_hash, email_verified, created_at, updated_at
         FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(map_user_row).transpose()
}

pub async fn update_user_password(pool: &PgPool, user_id: Uuid, password_hash: &str) -> Result<()> {
    sqlx::query(
        "UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1",
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn verify_user_email(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE users SET email_verified = true, updated_at = now() WHERE id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_refresh_token(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    expires_at: chrono::DateTime<chrono::Utc>,
    user_agent: Option<&str>,
    ip_hash: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, expires_at, user_agent, ip_hash)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(user_id)
    .bind(expires_at)
    .bind(user_agent)
    .bind(ip_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_refresh_token(pool: &PgPool, id: Uuid) -> Result<Option<RefreshTokenRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, expires_at, created_at, revoked_at
         FROM refresh_tokens WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(|r| Ok(RefreshTokenRow {
        id: r.try_get("id")?,
        user_id: r.try_get("user_id")?,
        expires_at: r.try_get("expires_at")?,
        created_at: r.try_get("created_at")?,
        revoked_at: r.try_get("revoked_at")?,
    }))
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

pub async fn revoke_all_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

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

    row.map(|r| Ok(EmailTokenRow {
        id: r.try_get("id")?,
        user_id: r.try_get("user_id")?,
        token_hash: r.try_get("token_hash")?,
        token_type: r.try_get("token_type")?,
        expires_at: r.try_get("expires_at")?,
        used_at: r.try_get("used_at")?,
    }))
    .transpose()
}

pub async fn mark_email_token_used(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE email_verification_tokens SET used_at = now() WHERE id = $1",
    )
    .bind(id)
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
    sqlx::query(
        "UPDATE orders SET user_id = $2 WHERE id = $1 AND user_id IS NULL",
    )
    .bind(order_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_order_user_id_tx(tx: &mut Transaction<'_, Postgres>, order_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE orders SET user_id = $2 WHERE id = $1 AND user_id IS NULL",
    )
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
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
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

pub async fn find_waitlist_entry_by_email(pool: &PgPool, email: &str) -> Result<Option<WaitlistEntryRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, email_verified, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term, verified_at, created_at
         FROM waitlist_entries WHERE LOWER(email) = LOWER($1)",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    row.map(|r| Ok(WaitlistEntryRow {
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
    })).transpose()
}

pub async fn find_waitlist_entry_by_referral_code(pool: &PgPool, code: &str) -> Result<Option<WaitlistEntryRow>> {
    let row = sqlx::query(
        "SELECT id, email, display_name, email_verified, referral_code, referred_by, ip_hash, utm_source, utm_medium, utm_campaign, utm_content, utm_term, verified_at, created_at
         FROM waitlist_entries WHERE referral_code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    row.map(|r| Ok(WaitlistEntryRow {
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
    })).transpose()
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

pub async fn find_waitlist_verify_token(pool: &PgPool, token_hash: &str) -> Result<Option<WaitlistVerifyTokenRow>> {
    let row = sqlx::query(
        "SELECT id, entry_id, expires_at, used_at FROM waitlist_verify_tokens WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    row.map(|r| Ok(WaitlistVerifyTokenRow {
        id: r.try_get("id")?,
        entry_id: r.try_get("entry_id")?,
        expires_at: r.try_get("expires_at")?,
        used_at: r.try_get("used_at")?,
    })).transpose()
}

pub async fn mark_waitlist_verify_token_used(pool: &PgPool, token_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE waitlist_verify_tokens SET used_at = now() WHERE id = $1",
    )
    .bind(token_id)
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
