use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbProvider {
    Postgres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    AwaitingPayment,
    PaymentDetected,
    PaymentConfirmed,
    UtilityDispatching,
    Completed,
    Expired,
    Failed,
    FlaggedForReview,
    Cancelled,
}

impl OrderStatus {
    pub fn as_db(&self) -> &'static str {
        match self {
            Self::AwaitingPayment => "awaiting_payment",
            Self::PaymentDetected => "payment_detected",
            Self::PaymentConfirmed => "payment_confirmed",
            Self::UtilityDispatching => "utility_dispatching",
            Self::Completed => "completed",
            Self::Expired => "expired",
            Self::Failed => "failed",
            Self::FlaggedForReview => "flagged_for_review",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::AwaitingPayment, Self::PaymentDetected)
                | (Self::AwaitingPayment, Self::Expired)
                | (Self::AwaitingPayment, Self::Cancelled)
                | (Self::PaymentDetected, Self::PaymentConfirmed)
                | (Self::PaymentDetected, Self::Expired)
                | (Self::PaymentDetected, Self::FlaggedForReview)
                | (Self::PaymentConfirmed, Self::UtilityDispatching)
                | (Self::UtilityDispatching, Self::Completed)
                | (Self::UtilityDispatching, Self::Failed)
        )
    }
}

#[derive(Debug, Clone)]
pub struct OrderStatusTransition {
    pub order_id: Uuid,
    pub from_status: OrderStatus,
    pub to_status: OrderStatus,
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
    transition: &OrderStatusTransition,
) -> Result<()> {
    if !transition
        .from_status
        .can_transition_to(transition.to_status)
    {
        return Err(anyhow!(
            "invalid status transition {} -> {}",
            transition.from_status.as_db(),
            transition.to_status.as_db()
        ));
    }

    let rows_affected = sqlx::query(
        "UPDATE orders
         SET status = $1
         WHERE id = $2
           AND status = $3",
    )
    .bind(transition.to_status.as_db())
    .bind(transition.order_id)
    .bind(transition.from_status.as_db())
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
    .bind(transition.from_status.as_db())
    .bind(transition.to_status.as_db())
    .bind(&transition.detail)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}
