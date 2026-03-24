use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use tokio::time::interval;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    domain::order::OrderStatus,
    http::handlers::HttpState,
    integrations::{
        rates::RateOracle,
        utility_provider::{ProviderTxnStatus, UtilityProvider, UtilityPurchaseRequest},
        vtpass::VtpassClient,
    },
    jobs::rate_refresher::RateRefresher,
    ws::WsOrderEvent,
};

pub fn start_background_workers(state: HttpState, config: AppConfig) {
    start_rate_refresher(state.clone(), config.clone());
    start_order_orchestrator(state, config);
}

fn start_rate_refresher(state: HttpState, config: AppConfig) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let oracle = match RateOracle::new(Duration::from_millis(config.rate_source_timeout_ms)) {
            Ok(oracle) => oracle,
            Err(error) => {
                tracing::error!(error = %error, "failed to initialize rate oracle");
                return;
            }
        };

        let refresher = RateRefresher::new(oracle.clone(), state.rate_cache.clone());
        let mut ticker = interval(Duration::from_secs(60));
        let mut pool = PgPoolOptions::new()
            .max_connections(3)
            .connect(&config.database_url)
            .await
            .ok();

        loop {
            ticker.tick().await;
            jobs.mark_alive("rate_refresher");

            let refreshed = if let Some(active_pool) = pool.as_ref() {
                refresher.refresh_once(active_pool).await
            } else {
                let previous = state.rate_cache.read().await.clone();
                match oracle.refresh(Some(&previous)).await {
                    Ok(result) => {
                        let mut cache = state.rate_cache.write().await;
                        *cache = result.current;
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            };

            if let Err(error) = refreshed {
                tracing::warn!(error = %error, "rate refresh iteration failed");
                if pool.is_none() {
                    pool = PgPoolOptions::new()
                        .max_connections(3)
                        .connect(&config.database_url)
                        .await
                        .ok();
                }
            }
        }
    });
}

fn start_order_orchestrator(state: HttpState, config: AppConfig) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let vtpass = match VtpassClient::from_config(&config) {
            Ok(client) => client,
            Err(error) => {
                tracing::error!(error = %error, "failed to initialize vtpass client");
                return;
            }
        };

        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            jobs.mark_alive("confirmation_watcher");
            jobs.mark_alive("utility_dispatcher");
            jobs.mark_alive("order_timeout_reaper");

            let order_ids = {
                let orders = state.orders.read().await;
                orders.keys().copied().collect::<Vec<_>>()
            };

            for order_id in order_ids {
                if let Err(error) = process_order(&state, &vtpass, order_id).await {
                    tracing::warn!(order_id = %order_id, error = %error, "order processing iteration failed");
                }
            }
        }
    });
}

async fn process_order(state: &HttpState, vtpass: &VtpassClient, order_id: Uuid) -> Result<()> {
    let now = Utc::now();

    let snapshot = {
        let orders = state.orders.read().await;
        match orders.get(&order_id) {
            Some(order) => order.clone(),
            None => return Ok(()),
        }
    };

    if matches!(
        snapshot.status,
        OrderStatus::Completed
            | OrderStatus::Failed
            | OrderStatus::Expired
            | OrderStatus::Cancelled
    ) {
        return Ok(());
    }

    if snapshot.status == OrderStatus::AwaitingPayment && snapshot.expires_at <= now {
        let mut orders = state.orders.write().await;
        if let Some(order) = orders.get_mut(&order_id) {
            if order.status == OrderStatus::AwaitingPayment {
                order.status = OrderStatus::Expired;
            }
        }
        drop(orders);
        let _ = state
            .ws_hub
            .broadcast_event(order_id, &WsOrderEvent::Expired)
            .await;
        return Ok(());
    }

    if matches!(
        snapshot.status,
        OrderStatus::AwaitingPayment | OrderStatus::PaymentDetected
    ) && state.zcash_rpc_client.is_some()
    {
        detect_payment_and_progress(state, order_id, &snapshot).await?;
    }

    let status_after_detection = {
        let orders = state.orders.read().await;
        orders.get(&order_id).map(|order| order.status)
    };

    if status_after_detection == Some(OrderStatus::PaymentConfirmed) {
        dispatch_utility(state, vtpass, order_id).await?;
    } else if status_after_detection == Some(OrderStatus::UtilityDispatching) {
        requery_utility_dispatch(state, vtpass, order_id).await?;
    }

    Ok(())
}

async fn detect_payment_and_progress(
    state: &HttpState,
    order_id: Uuid,
    snapshot: &crate::http::types::OrderRecord,
) -> Result<()> {
    let Some(client) = state.zcash_rpc_client.as_ref() else {
        return Ok(());
    };

    let notes = client
        .list_received_by_address(&snapshot.deposit_address, 0)
        .await?;
    let total_received = notes
        .iter()
        .fold(Decimal::ZERO, |acc, note| acc + note.amount);
    if total_received <= Decimal::ZERO {
        return Ok(());
    }

    let confirmations = notes
        .iter()
        .map(|note| note.confirmations)
        .max()
        .unwrap_or(0);
    let confirmations_u16 = u16::try_from(confirmations).unwrap_or(u16::MAX);

    let mut events = Vec::new();
    {
        let mut orders = state.orders.write().await;
        if let Some(order) = orders.get_mut(&order_id) {
            if order.status == OrderStatus::AwaitingPayment {
                order.status = OrderStatus::PaymentDetected;
                events.push(WsOrderEvent::PaymentDetected {
                    confirmations: confirmations_u16,
                    required: order.required_confirmations,
                });
            }

            order.confirmations = confirmations_u16;
            order.total_received = Some(total_received);

            events.push(WsOrderEvent::Confirmation {
                confirmations: confirmations_u16,
                required: order.required_confirmations,
            });

            if confirmations_u16 >= order.required_confirmations {
                let underpay_threshold = order.zec_amount * Decimal::new(995, 3);
                if total_received < underpay_threshold {
                    order.status = OrderStatus::FlaggedForReview;
                    events.push(WsOrderEvent::Failed {
                        reason: String::from("underpaid_flagged"),
                    });
                } else {
                    order.status = OrderStatus::PaymentConfirmed;
                    events.push(WsOrderEvent::PaymentConfirmed {
                        confirmations: confirmations_u16,
                    });
                }
            }
        }
    }

    for event in events {
        let _ = state.ws_hub.broadcast_event(order_id, &event).await;
    }

    Ok(())
}

async fn dispatch_utility(state: &HttpState, vtpass: &VtpassClient, order_id: Uuid) -> Result<()> {
    let order = {
        let mut orders = state.orders.write().await;
        let Some(order) = orders.get_mut(&order_id) else {
            return Ok(());
        };
        if order.status != OrderStatus::PaymentConfirmed {
            return Ok(());
        }
        order.status = OrderStatus::UtilityDispatching;
        order.clone()
    };

    let _ = state
        .ws_hub
        .broadcast_event(order_id, &WsOrderEvent::Dispatching)
        .await;

    let response = vtpass
        .pay(&UtilityPurchaseRequest {
            order_id,
            request_id: order_id.to_string(),
            service_id: order.utility_slug.clone(),
            billers_code: order.service_ref.clone(),
            variation_code: None,
            amount_ngn: order.amount_ngn,
            phone: Some(order.service_ref.clone()),
            metadata: serde_json::json!({"utility_type": order.utility_type}),
            zec_amount: order.zec_amount,
        })
        .await;

    match response {
        Ok(result) if result.status == ProviderTxnStatus::Delivered => {
            complete_order(state, order_id, result.token).await;
        }
        Ok(result) if result.status == ProviderTxnStatus::Failed => {
            fail_order(state, order_id, "provider_failed").await;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(order_id = %order_id, error = %error, "utility dispatch failed");
        }
    }

    Ok(())
}

async fn requery_utility_dispatch(
    state: &HttpState,
    vtpass: &VtpassClient,
    order_id: Uuid,
) -> Result<()> {
    let response = vtpass.requery(&order_id.to_string()).await;

    match response {
        Ok(result) if result.status == ProviderTxnStatus::Delivered => {
            complete_order(state, order_id, result.token).await;
        }
        Ok(result) if result.status == ProviderTxnStatus::Failed => {
            fail_order(state, order_id, "provider_failed").await;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(order_id = %order_id, error = %error, "utility requery failed");
        }
    }

    Ok(())
}

async fn complete_order(state: &HttpState, order_id: Uuid, delivery_token: Option<String>) {
    let reference = {
        let mut orders = state.orders.write().await;
        let Some(order) = orders.get_mut(&order_id) else {
            return;
        };
        order.status = OrderStatus::Completed;
        order.completed_at = Some(Utc::now());
        order.delivery_token = delivery_token.clone();
        order.order_id.to_string()
    };

    let _ = state
        .ws_hub
        .broadcast_event(
            order_id,
            &WsOrderEvent::Completed {
                delivery_token,
                reference,
            },
        )
        .await;
}

async fn fail_order(state: &HttpState, order_id: Uuid, reason: &str) {
    {
        let mut orders = state.orders.write().await;
        if let Some(order) = orders.get_mut(&order_id) {
            order.status = OrderStatus::Failed;
        }
    }

    let _ = state
        .ws_hub
        .broadcast_event(
            order_id,
            &WsOrderEvent::Failed {
                reason: reason.to_owned(),
            },
        )
        .await;
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    #[test]
    fn underpay_threshold_matches_policy() {
        let expected = Decimal::new(100_000_000, 8);
        let threshold = expected * Decimal::new(995, 3);
        assert_eq!(threshold, Decimal::new(99_500_000, 8));
    }
}
