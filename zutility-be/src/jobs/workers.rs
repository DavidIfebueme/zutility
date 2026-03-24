use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::order::{OrderStatus, OrderStatusTransition, PaymentAmountState, ThresholdPolicy},
    integrations::utility_provider::{
        ProviderErrorKind, ProviderTxnStatus, UtilityProviderRouter, UtilityPurchaseRequest,
    },
};

#[derive(Debug, Clone)]
pub struct PaymentCheckOrder {
    pub order_id: Uuid,
    pub status: OrderStatus,
    pub expected_zec_amount: Decimal,
    pub required_confirmations: u16,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub metadata: serde_json::Value,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DispatchOrder {
    pub order_id: Uuid,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub zec_amount: Decimal,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ObservedPayment {
    pub total_received: Decimal,
    pub confirmations: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchExecution {
    Completed,
    RetryScheduled(Duration),
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkerIntervals {
    pub confirmation_watcher: Duration,
    pub utility_dispatcher: Duration,
    pub order_timeout_reaper: Duration,
    pub sweeper: Duration,
    pub rate_refresher: Duration,
}

impl Default for WorkerIntervals {
    fn default() -> Self {
        Self {
            confirmation_watcher: Duration::from_secs(75),
            utility_dispatcher: Duration::from_secs(30),
            order_timeout_reaper: Duration::from_secs(120),
            sweeper: Duration::from_secs(300),
            rate_refresher: Duration::from_secs(60),
        }
    }
}

#[async_trait]
pub trait WorkerOrderRepository: Send + Sync {
    async fn list_pending_non_expired_orders(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<PaymentCheckOrder>>;

    async fn record_payment_snapshot(
        &self,
        order_id: Uuid,
        total_received: Decimal,
        confirmations: u16,
    ) -> Result<()>;

    async fn apply_transition(
        &self,
        order_id: Uuid,
        transition: OrderStatusTransition,
        event: &str,
        detail: serde_json::Value,
    ) -> Result<()>;

    async fn list_awaiting_payment_to_expire(&self, now: DateTime<Utc>) -> Result<Vec<Uuid>>;

    async fn list_late_payment_detected(
        &self,
        now: DateTime<Utc>,
        grace_minutes: u16,
    ) -> Result<Vec<Uuid>>;

    async fn insert_sweep_audit(
        &self,
        txid: &str,
        amount_zec: Decimal,
        detail: serde_json::Value,
    ) -> Result<()>;
}

#[async_trait]
pub trait PaymentObserver: Send + Sync {
    async fn observe_payment(&self, order: &PaymentCheckOrder) -> Result<ObservedPayment>;
}

#[async_trait]
pub trait DispatchQueue: Send + Sync {
    async fn enqueue_utility_dispatch(&self, order_id: Uuid) -> Result<bool>;
}

#[async_trait]
pub trait SweeperGateway: Send + Sync {
    async fn hot_wallet_balance(&self) -> Result<Decimal>;
    async fn submit_sweep(&self, amount: Decimal) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct ConfirmationWatcher {
    thresholds: ThresholdPolicy,
}

impl ConfirmationWatcher {
    pub fn new(thresholds: ThresholdPolicy) -> Self {
        Self { thresholds }
    }

    pub fn interval() -> Duration {
        WorkerIntervals::default().confirmation_watcher
    }

    pub async fn run_once(
        &self,
        repository: &dyn WorkerOrderRepository,
        payment_observer: &dyn PaymentObserver,
        queue: &dyn DispatchQueue,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let pending_orders = repository.list_pending_non_expired_orders(now).await?;

        for order in pending_orders {
            let observed = payment_observer.observe_payment(&order).await?;
            repository
                .record_payment_snapshot(
                    order.order_id,
                    observed.total_received,
                    observed.confirmations,
                )
                .await?;

            if observed.total_received <= Decimal::ZERO {
                continue;
            }

            if matches!(order.status, OrderStatus::AwaitingPayment) {
                let transition = OrderStatusTransition::new(
                    OrderStatus::AwaitingPayment,
                    OrderStatus::PaymentDetected,
                )
                .context("invalid awaiting_payment -> payment_detected transition")?;
                repository
                    .apply_transition(
                        order.order_id,
                        transition,
                        "payment_detected",
                        json!({
                            "confirmations": observed.confirmations,
                            "total_received": observed.total_received.to_string(),
                        }),
                    )
                    .await?;
            }

            let classification = self
                .thresholds
                .classify_received_amount(order.expected_zec_amount, observed.total_received);

            if matches!(classification, PaymentAmountState::Underpaid)
                && observed.confirmations >= order.required_confirmations
            {
                let transition = OrderStatusTransition::new(
                    OrderStatus::PaymentDetected,
                    OrderStatus::FlaggedForReview,
                )
                .context("invalid payment_detected -> flagged_for_review transition")?;
                repository
                    .apply_transition(
                        order.order_id,
                        transition,
                        "underpaid_flagged",
                        json!({
                            "expected": order.expected_zec_amount.to_string(),
                            "received": observed.total_received.to_string(),
                        }),
                    )
                    .await?;
                continue;
            }

            if observed.confirmations >= order.required_confirmations
                && matches!(
                    classification,
                    PaymentAmountState::InRange | PaymentAmountState::Overpaid
                )
            {
                let transition = OrderStatusTransition::new(
                    OrderStatus::PaymentDetected,
                    OrderStatus::PaymentConfirmed,
                )
                .context("invalid payment_detected -> payment_confirmed transition")?;
                repository
                    .apply_transition(
                        order.order_id,
                        transition,
                        "payment_confirmed",
                        json!({
                            "confirmations": observed.confirmations,
                            "classification": match classification {
                                PaymentAmountState::InRange => "in_range",
                                PaymentAmountState::Overpaid => "overpaid",
                                PaymentAmountState::Underpaid => "underpaid",
                            },
                        }),
                    )
                    .await?;

                let _ = queue.enqueue_utility_dispatch(order.order_id).await?;
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct UtilityDispatcher {
    provider_router: UtilityProviderRouter,
}

impl UtilityDispatcher {
    pub fn new(provider_router: UtilityProviderRouter) -> Self {
        Self { provider_router }
    }

    pub fn retry_backoff(attempt: u8) -> Option<Duration> {
        match attempt {
            0 => Some(Duration::from_secs(30)),
            1 => Some(Duration::from_secs(120)),
            2 => Some(Duration::from_secs(600)),
            _ => None,
        }
    }

    pub async fn dispatch_order(
        &self,
        repository: &dyn WorkerOrderRepository,
        order: &DispatchOrder,
        attempt: u8,
    ) -> Result<DispatchExecution> {
        let to_dispatching = OrderStatusTransition::new(
            OrderStatus::PaymentConfirmed,
            OrderStatus::UtilityDispatching,
        )
        .context("invalid payment_confirmed -> utility_dispatching transition")?;
        repository
            .apply_transition(
                order.order_id,
                to_dispatching,
                "dispatch_started",
                json!({"attempt": attempt}),
            )
            .await?;

        let request = UtilityPurchaseRequest {
            order_id: order.order_id,
            request_id: order.order_id.to_string(),
            service_id: order.utility_slug.clone(),
            billers_code: order.service_ref.clone(),
            variation_code: None,
            amount_ngn: order.amount_ngn,
            phone: Some(order.service_ref.clone()),
            metadata: order.metadata.clone(),
            zec_amount: order.zec_amount,
        };

        match self.provider_router.pay(&request).await {
            Ok(response) if response.status == ProviderTxnStatus::Delivered => {
                let transition = OrderStatusTransition::new(
                    OrderStatus::UtilityDispatching,
                    OrderStatus::Completed,
                )
                .context("invalid utility_dispatching -> completed transition")?;
                repository
                    .apply_transition(
                        order.order_id,
                        transition,
                        "dispatch_completed",
                        json!({
                            "provider_request_id": response.provider_request_id,
                            "provider_reference": response.provider_reference,
                            "token": response.token,
                        }),
                    )
                    .await?;
                Ok(DispatchExecution::Completed)
            }
            Ok(_) => {
                let backoff = Self::retry_backoff(attempt).unwrap_or(Duration::from_secs(600));
                Ok(DispatchExecution::RetryScheduled(backoff))
            }
            Err(error)
                if matches!(
                    error.kind,
                    ProviderErrorKind::Transient | ProviderErrorKind::Outage
                ) =>
            {
                let backoff = Self::retry_backoff(attempt).unwrap_or(Duration::from_secs(600));
                Ok(DispatchExecution::RetryScheduled(backoff))
            }
            Err(error) => {
                let transition = OrderStatusTransition::new(
                    OrderStatus::UtilityDispatching,
                    OrderStatus::Failed,
                )
                .context("invalid utility_dispatching -> failed transition")?;
                repository
                    .apply_transition(
                        order.order_id,
                        transition,
                        "dispatch_failed",
                        json!({"reason": error.message}),
                    )
                    .await?;
                Ok(DispatchExecution::Failed)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderTimeoutReaper {
    grace_minutes: u16,
}

impl OrderTimeoutReaper {
    pub fn new(grace_minutes: u16) -> Self {
        Self { grace_minutes }
    }

    pub fn interval() -> Duration {
        WorkerIntervals::default().order_timeout_reaper
    }

    pub async fn run_once(
        &self,
        repository: &dyn WorkerOrderRepository,
        now: DateTime<Utc>,
    ) -> Result<()> {
        for order_id in repository.list_awaiting_payment_to_expire(now).await? {
            let transition =
                OrderStatusTransition::new(OrderStatus::AwaitingPayment, OrderStatus::Expired)
                    .context("invalid awaiting_payment -> expired transition")?;
            repository
                .apply_transition(
                    order_id,
                    transition,
                    "order_expired",
                    json!({ "expired_at": now }),
                )
                .await?;
        }

        for order_id in repository
            .list_late_payment_detected(now, self.grace_minutes)
            .await?
        {
            let transition = OrderStatusTransition::new(
                OrderStatus::PaymentDetected,
                OrderStatus::FlaggedForReview,
            )
            .context("invalid payment_detected -> flagged_for_review transition")?;
            repository
                .apply_transition(
                    order_id,
                    transition,
                    "late_payment_review",
                    json!({"grace_minutes": self.grace_minutes}),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SweeperJob {
    threshold_zec: Decimal,
}

impl SweeperJob {
    pub fn new(threshold_zec: Decimal) -> Self {
        Self { threshold_zec }
    }

    pub fn interval() -> Duration {
        WorkerIntervals::default().sweeper
    }

    pub async fn run_once(
        &self,
        repository: &dyn WorkerOrderRepository,
        sweeper: &dyn SweeperGateway,
    ) -> Result<Option<String>> {
        let balance = sweeper.hot_wallet_balance().await?;
        if balance <= self.threshold_zec {
            return Ok(None);
        }

        let txid = sweeper.submit_sweep(balance).await?;
        repository
            .insert_sweep_audit(
                &txid,
                balance,
                json!({
                    "threshold": self.threshold_zec.to_string(),
                    "swept_amount": balance.to_string(),
                }),
            )
            .await?;
        Ok(Some(txid))
    }
}
