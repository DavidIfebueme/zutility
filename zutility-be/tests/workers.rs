use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use tokio::sync::Mutex;
use uuid::Uuid;
use zutility_be::{
    domain::order::{OrderStatus, OrderStatusTransition, ThresholdPolicy},
    integrations::utility_provider::{
        ProviderError, ProviderErrorKind, ProviderKind, ProviderTxnStatus, RequeryResponse,
        UtilityProvider, UtilityProviderRouter, UtilityPurchaseRequest, UtilityPurchaseResponse,
        UtilityVariation, ValidateRefRequest, ValidateRefResponse,
    },
    jobs::workers::{
        ConfirmationWatcher, DispatchExecution, DispatchOrder, DispatchQueue, ObservedPayment,
        OrderTimeoutReaper, PaymentCheckOrder, PaymentObserver, SweeperGateway, SweeperJob,
        UtilityDispatcher, WorkerOrderRepository,
    },
};

#[derive(Default)]
struct RepoState {
    pending: Vec<PaymentCheckOrder>,
    expiring: Vec<Uuid>,
    late: Vec<Uuid>,
    transitions: Vec<(Uuid, OrderStatus, OrderStatus, String)>,
    snapshots: Vec<(Uuid, Decimal, u16)>,
    sweeps: Vec<(String, Decimal)>,
}

#[derive(Clone, Default)]
struct MockRepo {
    state: Arc<Mutex<RepoState>>,
}

#[async_trait]
impl WorkerOrderRepository for MockRepo {
    async fn list_pending_non_expired_orders(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PaymentCheckOrder>> {
        Ok(self.state.lock().await.pending.clone())
    }

    async fn record_payment_snapshot(
        &self,
        order_id: Uuid,
        total_received: Decimal,
        confirmations: u16,
    ) -> Result<()> {
        self.state
            .lock()
            .await
            .snapshots
            .push((order_id, total_received, confirmations));
        Ok(())
    }

    async fn apply_transition(
        &self,
        order_id: Uuid,
        transition: OrderStatusTransition,
        event: &str,
        _detail: serde_json::Value,
    ) -> Result<()> {
        self.state.lock().await.transitions.push((
            order_id,
            transition.from,
            transition.to,
            event.to_owned(),
        ));
        Ok(())
    }

    async fn list_awaiting_payment_to_expire(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Uuid>> {
        Ok(self.state.lock().await.expiring.clone())
    }

    async fn list_late_payment_detected(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
        _grace_minutes: u16,
    ) -> Result<Vec<Uuid>> {
        Ok(self.state.lock().await.late.clone())
    }

    async fn insert_sweep_audit(
        &self,
        txid: &str,
        amount_zec: Decimal,
        _detail: serde_json::Value,
    ) -> Result<()> {
        self.state
            .lock()
            .await
            .sweeps
            .push((txid.to_owned(), amount_zec));
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MockObserver {
    payments: Arc<Mutex<HashMap<Uuid, ObservedPayment>>>,
}

#[async_trait]
impl PaymentObserver for MockObserver {
    async fn observe_payment(&self, order: &PaymentCheckOrder) -> Result<ObservedPayment> {
        Ok(self
            .payments
            .lock()
            .await
            .get(&order.order_id)
            .cloned()
            .unwrap_or(ObservedPayment {
                total_received: Decimal::ZERO,
                confirmations: 0,
            }))
    }
}

#[derive(Clone, Default)]
struct MockQueue {
    enqueued: Arc<Mutex<HashSet<Uuid>>>,
}

#[async_trait]
impl DispatchQueue for MockQueue {
    async fn enqueue_utility_dispatch(&self, order_id: Uuid) -> Result<bool> {
        Ok(self.enqueued.lock().await.insert(order_id))
    }
}

#[derive(Clone)]
struct MockSweeper {
    balance: Decimal,
    txid: String,
}

#[async_trait]
impl SweeperGateway for MockSweeper {
    async fn hot_wallet_balance(&self) -> Result<Decimal> {
        Ok(self.balance)
    }

    async fn submit_sweep(&self, _amount: Decimal) -> Result<String> {
        Ok(self.txid.clone())
    }
}

#[derive(Clone)]
struct MockProvider {
    status: ProviderTxnStatus,
    error_kind: Option<ProviderErrorKind>,
}

#[derive(Clone, Default)]
struct RetryTrackingProvider {
    seen_request_ids: Arc<Mutex<Vec<String>>>,
    calls: Arc<Mutex<u8>>,
}

#[async_trait]
impl UtilityProvider for RetryTrackingProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vtpass
    }

    async fn service_variations(
        &self,
        _service_id: &str,
    ) -> std::result::Result<Vec<UtilityVariation>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_reference(
        &self,
        _request: &ValidateRefRequest,
    ) -> std::result::Result<ValidateRefResponse, ProviderError> {
        Ok(ValidateRefResponse {
            is_valid: true,
            customer_name: None,
            raw: serde_json::json!({}),
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> std::result::Result<UtilityPurchaseResponse, ProviderError> {
        self.seen_request_ids
            .lock()
            .await
            .push(request.request_id.clone());

        let mut calls = self.calls.lock().await;
        *calls = calls.saturating_add(1);
        if *calls == 1 {
            return Err(ProviderError::transient("temporary provider failure"));
        }

        Ok(UtilityPurchaseResponse {
            provider_reference: request.order_id.to_string(),
            provider_request_id: request.request_id.clone(),
            status: ProviderTxnStatus::Delivered,
            token: None,
            raw: serde_json::json!({}),
        })
    }

    async fn requery(
        &self,
        request_id: &str,
    ) -> std::result::Result<RequeryResponse, ProviderError> {
        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status: ProviderTxnStatus::Pending,
            token: None,
            raw: serde_json::json!({}),
        })
    }

    fn verify_webhook_signature(&self, _payload: &[u8], _signature: &str) -> bool {
        true
    }

    fn parse_webhook_event(
        &self,
        _payload: &[u8],
    ) -> std::result::Result<
        zutility_be::integrations::utility_provider::ProviderWebhookEvent,
        ProviderError,
    > {
        Ok(
            zutility_be::integrations::utility_provider::ProviderWebhookEvent {
                provider_request_id: String::from("req"),
                status: ProviderTxnStatus::Pending,
                token: None,
                raw: serde_json::json!({}),
            },
        )
    }
}

#[async_trait]
impl UtilityProvider for MockProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vtpass
    }

    async fn service_variations(
        &self,
        _service_id: &str,
    ) -> std::result::Result<Vec<UtilityVariation>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_reference(
        &self,
        _request: &ValidateRefRequest,
    ) -> std::result::Result<ValidateRefResponse, ProviderError> {
        Ok(ValidateRefResponse {
            is_valid: true,
            customer_name: None,
            raw: serde_json::json!({}),
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> std::result::Result<UtilityPurchaseResponse, ProviderError> {
        if let Some(kind) = self.error_kind {
            return Err(ProviderError::new(kind, "dispatch error"));
        }
        Ok(UtilityPurchaseResponse {
            provider_reference: request.order_id.to_string(),
            provider_request_id: request.request_id.clone(),
            status: self.status,
            token: None,
            raw: serde_json::json!({}),
        })
    }

    async fn requery(
        &self,
        request_id: &str,
    ) -> std::result::Result<RequeryResponse, ProviderError> {
        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status: self.status,
            token: None,
            raw: serde_json::json!({}),
        })
    }

    fn verify_webhook_signature(&self, _payload: &[u8], _signature: &str) -> bool {
        true
    }

    fn parse_webhook_event(
        &self,
        _payload: &[u8],
    ) -> std::result::Result<
        zutility_be::integrations::utility_provider::ProviderWebhookEvent,
        ProviderError,
    > {
        Ok(
            zutility_be::integrations::utility_provider::ProviderWebhookEvent {
                provider_request_id: String::from("req"),
                status: self.status,
                token: None,
                raw: serde_json::json!({}),
            },
        )
    }
}

#[tokio::test]
async fn confirmation_watcher_confirms_and_enqueues_once() {
    let order_id = Uuid::new_v4();
    let repo = MockRepo::default();
    repo.state.lock().await.pending.push(PaymentCheckOrder {
        order_id,
        status: OrderStatus::AwaitingPayment,
        expected_zec_amount: Decimal::new(100_000_000, 8),
        required_confirmations: 3,
        utility_slug: String::from("mtn"),
        service_ref: String::from("08000000000"),
        amount_ngn: 1000,
        metadata: serde_json::json!({}),
        expires_at: Utc::now() + Duration::minutes(10),
    });

    let observer = MockObserver::default();
    observer.payments.lock().await.insert(
        order_id,
        ObservedPayment {
            total_received: Decimal::new(101_000_000, 8),
            confirmations: 3,
        },
    );

    let queue = MockQueue::default();
    let watcher = ConfirmationWatcher::new(ThresholdPolicy::default());
    let result = watcher.run_once(&repo, &observer, &queue, Utc::now()).await;
    assert!(result.is_ok());

    let transitions = repo.state.lock().await.transitions.clone();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].1, OrderStatus::AwaitingPayment);
    assert_eq!(transitions[0].2, OrderStatus::PaymentDetected);
    assert_eq!(transitions[1].1, OrderStatus::PaymentDetected);
    assert_eq!(transitions[1].2, OrderStatus::PaymentConfirmed);
    assert!(queue.enqueued.lock().await.contains(&order_id));
}

#[tokio::test]
async fn confirmation_watcher_flags_underpaid_after_required_confirmations() {
    let order_id = Uuid::new_v4();
    let repo = MockRepo::default();
    repo.state.lock().await.pending.push(PaymentCheckOrder {
        order_id,
        status: OrderStatus::AwaitingPayment,
        expected_zec_amount: Decimal::new(100_000_000, 8),
        required_confirmations: 3,
        utility_slug: String::from("mtn"),
        service_ref: String::from("08000000000"),
        amount_ngn: 1000,
        metadata: serde_json::json!({}),
        expires_at: Utc::now() + Duration::minutes(10),
    });

    let observer = MockObserver::default();
    observer.payments.lock().await.insert(
        order_id,
        ObservedPayment {
            total_received: Decimal::new(90_000_000, 8),
            confirmations: 3,
        },
    );

    let queue = MockQueue::default();
    let watcher = ConfirmationWatcher::new(ThresholdPolicy::default());
    let result = watcher.run_once(&repo, &observer, &queue, Utc::now()).await;
    assert!(result.is_ok());

    let transitions = repo.state.lock().await.transitions.clone();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[1].2, OrderStatus::FlaggedForReview);
}

#[tokio::test]
async fn utility_dispatcher_handles_completed_retry_and_failed_paths() {
    let order = DispatchOrder {
        order_id: Uuid::new_v4(),
        utility_slug: String::from("mtn"),
        service_ref: String::from("08000000000"),
        amount_ngn: 1000,
        zec_amount: Decimal::new(1, 0),
        metadata: serde_json::json!({}),
    };

    let completed_router = UtilityProviderRouter::new(Arc::new(MockProvider {
        status: ProviderTxnStatus::Delivered,
        error_kind: None,
    }));
    let completed = UtilityDispatcher::new(completed_router)
        .dispatch_order(&MockRepo::default(), &order, 0)
        .await;
    assert!(matches!(completed, Ok(DispatchExecution::Completed)));

    let retry_router = UtilityProviderRouter::new(Arc::new(MockProvider {
        status: ProviderTxnStatus::Pending,
        error_kind: None,
    }));
    let retry = UtilityDispatcher::new(retry_router)
        .dispatch_order(&MockRepo::default(), &order, 1)
        .await;
    assert!(matches!(retry, Ok(DispatchExecution::RetryScheduled(_))));

    let failed_router = UtilityProviderRouter::new(Arc::new(MockProvider {
        status: ProviderTxnStatus::Failed,
        error_kind: Some(ProviderErrorKind::Permanent),
    }));
    let failed_repo = MockRepo::default();
    let failed = UtilityDispatcher::new(failed_router)
        .dispatch_order(&failed_repo, &order, 0)
        .await;
    assert!(matches!(failed, Ok(DispatchExecution::Failed)));
    let transitions = failed_repo.state.lock().await.transitions.clone();
    assert!(transitions.iter().any(|item| item.2 == OrderStatus::Failed));
}

#[tokio::test]
async fn timeout_reaper_expires_and_flags_late_orders() {
    let repo = MockRepo::default();
    let expiring_id = Uuid::new_v4();
    let late_id = Uuid::new_v4();
    {
        let mut state = repo.state.lock().await;
        state.expiring.push(expiring_id);
        state.late.push(late_id);
    }

    let reaper = OrderTimeoutReaper::new(120);
    let result = reaper.run_once(&repo, Utc::now()).await;
    assert!(result.is_ok());

    let transitions = repo.state.lock().await.transitions.clone();
    assert!(
        transitions
            .iter()
            .any(|item| item.0 == expiring_id && item.2 == OrderStatus::Expired)
    );
    assert!(
        transitions
            .iter()
            .any(|item| item.0 == late_id && item.2 == OrderStatus::FlaggedForReview)
    );
}

#[tokio::test]
async fn sweeper_runs_only_above_threshold() {
    let repo = MockRepo::default();
    let job = SweeperJob::new(Decimal::new(5, 0));

    let low = job
        .run_once(
            &repo,
            &MockSweeper {
                balance: Decimal::new(4, 0),
                txid: String::from("low"),
            },
        )
        .await;
    assert!(matches!(low, Ok(None)));

    let high = job
        .run_once(
            &repo,
            &MockSweeper {
                balance: Decimal::new(7, 0),
                txid: String::from("tx-123"),
            },
        )
        .await;
    assert!(matches!(high, Ok(Some(_))));
    assert_eq!(repo.state.lock().await.sweeps.len(), 1);
}

#[tokio::test]
async fn utility_dispatcher_retries_with_same_idempotency_request_id() {
    let order = DispatchOrder {
        order_id: Uuid::new_v4(),
        utility_slug: String::from("mtn"),
        service_ref: String::from("08000000000"),
        amount_ngn: 1500,
        zec_amount: Decimal::new(12, 1),
        metadata: serde_json::json!({}),
    };

    let provider = Arc::new(RetryTrackingProvider::default());
    let router = UtilityProviderRouter::new(provider.clone());
    let dispatcher = UtilityDispatcher::new(router);
    let repo = MockRepo::default();

    let first = dispatcher.dispatch_order(&repo, &order, 0).await;
    assert!(matches!(first, Ok(DispatchExecution::RetryScheduled(_))));

    let second = dispatcher.dispatch_order(&repo, &order, 1).await;
    assert!(matches!(second, Ok(DispatchExecution::Completed)));

    let seen = provider.seen_request_ids.lock().await.clone();
    assert_eq!(seen.len(), 2);
    assert!(
        seen.iter()
            .all(|request_id| request_id == &order.order_id.to_string())
    );
}
