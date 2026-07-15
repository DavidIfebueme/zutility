use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::time::interval;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    db,
    domain::order::OrderStatus,
    http::handlers::HttpState,
    integrations::{
        provider_dispatcher::ProviderDispatcher,
        rates::RateOracle,
        remita::RemitaClient,
        utility_provider::{ProviderTxnStatus, UtilityPurchaseRequest},
        vtpass::VtpassClient,
    },
    jobs::{address_pool::AddressPoolManager, rate_refresher::RateRefresher},
    ws::WsOrderEvent,
};

pub fn start_background_workers(state: HttpState, config: AppConfig, pool: PgPool) {
    start_rate_refresher(state.clone(), config.clone(), pool.clone());
    start_order_orchestrator(state.clone(), config.clone(), pool.clone());
    start_address_pool_refill(state, config, pool);
}

fn start_rate_refresher(state: HttpState, config: AppConfig, pool: PgPool) {
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

        loop {
            ticker.tick().await;
            jobs.mark_alive("rate_refresher");

            if let Err(error) = refresher.refresh_once(&pool).await {
                tracing::warn!(error = %error, "rate refresh iteration failed");
            }
        }
    });
}

fn start_order_orchestrator(state: HttpState, config: AppConfig, pool: PgPool) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let vtpass = match VtpassClient::from_config(&config) {
            Ok(client) => client,
            Err(error) => {
                tracing::error!(error = %error, "failed to initialize vtpass client");
                return;
            }
        };

        let remita = match RemitaClient::from_config(&config) {
            Ok(client) => {
                tracing::info!("remita client initialized successfully");
                Some(client)
            }
            Err(error) => {
                tracing::warn!(error = %error, "remita client not configured — school fees and electricity fallback will be unavailable");
                None
            }
        };

        let inlomax = match crate::integrations::inlomax::InlomaxClient::from_config(&config) {
            Ok(client) => {
                tracing::info!("inlomax client initialized successfully");
                Some(client)
            }
            Err(error) => {
                tracing::warn!(error = %error, "inlomax client not configured — airtime/data/cable/electricity/education will use vtpass fallback");
                None
            }
        };

        let dispatcher = ProviderDispatcher::new(vtpass, remita, inlomax);

        let mut ticker = interval(Duration::from_secs(60));
        let mut consecutive_sync_failures: u32 = 0;
        loop {
            ticker.tick().await;
            jobs.mark_alive("confirmation_watcher");
            jobs.mark_alive("utility_dispatcher");
            jobs.mark_alive("order_timeout_reaper");

            if let Some(client) = state.zcash_client.as_ref() {
                match client.sync().await {
                    Ok(()) => {
                        if consecutive_sync_failures > 0 {
                            tracing::info!(
                                previous_failures = consecutive_sync_failures,
                                "zingolib sync recovered after failures"
                            );
                        }
                        consecutive_sync_failures = 0;
                    }
                    Err(error) => {
                        consecutive_sync_failures += 1;
                        if consecutive_sync_failures >= 3 {
                            tracing::error!(
                                consecutive_failures = consecutive_sync_failures,
                                error = %error,
                                "zingolib sync has failed consecutively — indexer may be down"
                            );
                        } else {
                            tracing::warn!(
                                consecutive_failures = consecutive_sync_failures,
                                error = %error,
                                "zingolib sync failed during order processing cycle"
                            );
                        }
                    }
                }
            }

            match db::list_active_orders(&pool).await {
                Ok(orders) => {
                    for order in orders {
                        if let Err(error) = process_order(&state, &dispatcher, &pool, &order).await {
                            tracing::warn!(order_id = %order.id, error = %error, "order processing iteration failed");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "failed to list active orders");
                }
            }

            if let Some(client) = state.zcash_client.as_ref() {
                if let Err(error) = client.save_wallet().await {
                    tracing::warn!(error = %error, "zingolib wallet save failed");
                }
            }
        }
    });
}

async fn process_order(
    state: &HttpState,
    dispatcher: &ProviderDispatcher,
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<()> {
    let now = Utc::now();
    let order_status = order.status()?;

    if matches!(
        order_status,
        OrderStatus::Completed
            | OrderStatus::Failed
            | OrderStatus::Expired
            | OrderStatus::Cancelled
    ) {
        return Ok(());
    }

    if order_status == OrderStatus::AwaitingPayment && order.expires_at <= now {
        db::update_order_status_cas(
            pool,
            order.id,
            "awaiting_payment",
            "expired",
            "order_expired",
            serde_json::json!({ "expired_at": now }),
        )
        .await?;
        let _ = state
            .ws_hub
            .broadcast_event(order.id, &WsOrderEvent::Expired)
            .await;
        notify_order(
            pool,
            order,
            "order_expired",
            "Order Expired",
            &format!("Your {} order has expired.", order.utility_slug),
            serde_json::json!({ "utility_slug": order.utility_slug }),
        );
        return Ok(());
    }

    if matches!(
        order_status,
        OrderStatus::AwaitingPayment | OrderStatus::PaymentDetected
    ) && state.zcash_client.is_some()
    {
        detect_payment_and_progress(state, pool, order).await?;
    }

    let refreshed = db::get_order_by_id(pool, order.id).await?;
    let Some(updated) = refreshed else {
        return Ok(());
    };
    let updated_status = updated.status()?;

    if updated_status == OrderStatus::PaymentConfirmed {
        dispatch_utility(state, dispatcher, pool, &updated).await?;
    } else if updated_status == OrderStatus::UtilityDispatching {
        requery_utility_dispatch(state, dispatcher, pool, &updated).await?;
    }

    Ok(())
}

async fn detect_payment_and_progress(
    state: &HttpState,
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<()> {
    let Some(client) = state.zcash_client.as_ref() else {
        return Ok(());
    };

    let since_ts = order.created_at.timestamp() as u32;

    let (total_received, confirmations) = if order.address_type == "transparent" {
        let chain_info = client.get_blockchain_info().await.ok();
        let current_height = chain_info.map(|info| info.blocks).unwrap_or(0);
        let observation = client
            .observe_transparent_payment(&order.deposit_address, current_height, since_ts)
            .await?;
        (observation.total_received, observation.confirmations)
    } else {
        let notes = client
            .list_received_by_address(&order.deposit_address, 0, since_ts)
            .await?;
        let total = notes
            .iter()
            .fold(Decimal::ZERO, |acc, note| acc + note.amount);
        let confs = notes
            .iter()
            .map(|note| note.confirmations)
            .max()
            .unwrap_or(0);
        (
            total,
            u16::try_from(confs).unwrap_or(u16::MAX),
        )
    };

    if total_received <= Decimal::ZERO {
        return Ok(());
    }

    let confirmations_i32 = i32::from(confirmations);
    let required_confs = order.required_confs;

    db::update_order_payment(pool, order.id, confirmations_i32, total_received, None).await?;

    let mut events = Vec::new();
    let order_status = order.status()?;

    if order_status == OrderStatus::AwaitingPayment {
        db::update_order_status_cas(
            pool,
            order.id,
            "awaiting_payment",
            "payment_detected",
            "payment_detected",
            serde_json::json!({
                "confirmations": confirmations,
                "total_received": total_received.to_string(),
            }),
        )
        .await?;
        events.push(WsOrderEvent::PaymentDetected {
            confirmations,
            required: u16::try_from(required_confs).unwrap_or(u16::MAX),
        });
        notify_order(
            pool,
            order,
            "payment_detected",
            "Payment Detected",
            &format!("We detected your ZEC payment for {}.", order.utility_slug),
            serde_json::json!({ "utility_slug": order.utility_slug, "confirmations": confirmations }),
        );
    }

    events.push(WsOrderEvent::Confirmation {
        confirmations,
        required: u16::try_from(required_confs).unwrap_or(u16::MAX),
    });

    if confirmations_i32 >= required_confs {
        let threshold = order.zec_amount * Decimal::new(995, 3);
        if total_received < threshold {
            db::update_order_status_cas(
                pool,
                order.id,
                "payment_detected",
                "flagged_for_review",
                "underpaid_flagged",
                serde_json::json!({
                    "expected": order.zec_amount.to_string(),
                    "received": total_received.to_string(),
                }),
            )
            .await?;
            events.push(WsOrderEvent::Failed {
                reason: String::from("underpaid_flagged"),
            });
            notify_order(
                pool,
                order,
                "order_flagged",
                "Order Under Review",
                &format!("Your {} payment is being reviewed due to an underpayment.", order.utility_slug),
                serde_json::json!({ "utility_slug": order.utility_slug }),
            );
        } else {
            db::update_order_status_cas(
                pool,
                order.id,
                "payment_detected",
                "payment_confirmed",
                "payment_confirmed",
                serde_json::json!({
                    "confirmations": confirmations,
                }),
            )
            .await?;
            events.push(WsOrderEvent::PaymentConfirmed { confirmations });
            notify_order(
                pool,
                order,
                "payment_confirmed",
                "Payment Confirmed",
                &format!("Your payment for {} has been confirmed. Processing your order...", order.utility_slug),
                serde_json::json!({ "utility_slug": order.utility_slug }),
            );
        }
    }

    for event in events {
        let _ = state.ws_hub.broadcast_event(order.id, &event).await;
    }

    Ok(())
}

async fn dispatch_utility(
    state: &HttpState,
    dispatcher: &ProviderDispatcher,
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<()> {
    let provider_kind = dispatcher.provider_kind_for(&order.utility_type);
    let provider_name = format!("{provider_kind:?}").to_lowercase();

    let request_id = match provider_kind {
        crate::integrations::utility_provider::ProviderKind::Remita => {
            format!("rm-{}", order.id.as_simple())
        }
        crate::integrations::utility_provider::ProviderKind::Inlomax => {
            format!("inl-{}", order.id.as_simple())
        }
        _ => format!("vp-{}", order.id.as_simple()),
    };

    let transitioned = db::set_order_dispatching(pool, order.id, &provider_name, Some(&request_id)).await?;
    if !transitioned {
        return Ok(());
    }

    let _ = state
        .ws_hub
        .broadcast_event(order.id, &WsOrderEvent::Dispatching)
        .await;

    notify_order(
        pool,
        order,
        "utility_dispatching",
        "Processing Your Order",
        &format!("We're sending your {} payment for processing.", order.utility_slug),
        serde_json::json!({ "utility_slug": order.utility_slug }),
    );

    let response = dispatcher
        .pay(
            &order.utility_type,
            &UtilityPurchaseRequest {
                order_id: order.id,
                request_id: request_id.clone(),
                service_id: order.utility_slug.clone(),
                billers_code: order.service_ref.clone(),
                variation_code: order.variation_code.clone(),
                amount_ngn: order.amount_ngn,
                phone: Some(order.service_ref.clone()),
                metadata: serde_json::json!({"utility_type": order.utility_type, "customer_name": order.customer_name}),
                zec_amount: order.zec_amount,
            },
        )
        .await;

    match response {
        Ok(result) if result.status == ProviderTxnStatus::Delivered => {
            complete_order(state, pool, order.id, result.token.as_deref(), Some(result.provider_request_id.as_str())).await;
        }
        Ok(result) if result.status == ProviderTxnStatus::Failed => {
            fail_order(state, pool, order.id, "provider_failed").await;
        }
        Ok(result) => {
            if !result.provider_request_id.is_empty() {
                if let Err(error) = db::store_provider_reference(pool, order.id, &result.provider_request_id).await {
                    tracing::warn!(order_id = %order.id, error = %error, "failed to store provider reference for requery");
                }
            }
        }
        Err(error) => {
            tracing::warn!(order_id = %order.id, error = %error, "utility dispatch failed");
        }
    }

    Ok(())
}

async fn requery_utility_dispatch(
    state: &HttpState,
    dispatcher: &ProviderDispatcher,
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<()> {
    let fallback_ref = order.id.to_string();
    let reference = order.vtpass_request_id.as_deref().unwrap_or(&fallback_ref);
    if order.vtpass_request_id.is_none() {
        tracing::warn!(order_id = %order.id, "requery has no stored provider reference, falling back to order ID");
    }
    let response = dispatcher.requery(&order.utility_type, reference).await;

    match response {
        Ok(result) if result.status == ProviderTxnStatus::Delivered => {
            complete_order(state, pool, order.id, result.token.as_deref(), Some(result.provider_request_id.as_str())).await;
        }
        Ok(result) if result.status == ProviderTxnStatus::Failed => {
            fail_order(state, pool, order.id, "provider_failed").await;
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(order_id = %order.id, error = %error, "utility requery failed");
        }
    }

    Ok(())
}

fn notify_order(
    pool: &PgPool,
    order: &db::OrderRow,
    notification_type: &str,
    title: &str,
    body: &str,
    detail: serde_json::Value,
) {
    let user_id = match order.user_id {
        Some(uid) => uid,
        None => return,
    };
    let pool = pool.clone();
    let order_id = order.id;
    let nt = notification_type.to_owned();
    let t = title.to_owned();
    let b = body.to_owned();
    tokio::spawn(async move {
        if let Err(e) = db::create_notification(&pool, user_id, Some(order_id), &nt, &t, &b, detail).await {
            tracing::warn!(order_id = %order_id, error = %e, "failed to create notification");
        }
    });
}

async fn complete_order(
    state: &HttpState,
    pool: &PgPool,
    order_id: Uuid,
    delivery_token: Option<&str>,
    vtpass_request_id: Option<&str>,
) {
    if let Err(error) = db::complete_order(pool, order_id, delivery_token, vtpass_request_id).await {
        tracing::warn!(order_id = %order_id, error = %error, "failed to complete order in db");
        return;
    }

    let _ = state
        .ws_hub
        .broadcast_event(
            order_id,
            &WsOrderEvent::Completed {
                delivery_token: delivery_token.map(ToOwned::to_owned),
                reference: order_id.to_string(),
            },
        )
        .await;

    if let Ok(Some(order)) = db::get_order_by_id(pool, order_id).await {
        notify_order(
            pool,
            &order,
            "order_completed",
            "Order Completed",
            &format!("Your {} payment was delivered successfully!", order.utility_slug),
            serde_json::json!({ "utility_slug": order.utility_slug }),
        );
    }
}

async fn fail_order(state: &HttpState, pool: &PgPool, order_id: Uuid, reason: &str) {
    if let Err(error) = db::fail_order(pool, order_id).await {
        tracing::warn!(order_id = %order_id, error = %error, "failed to mark order as failed in db");
        return;
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

    if let Ok(Some(order)) = db::get_order_by_id(pool, order_id).await {
        notify_order(
            pool,
            &order,
            "order_failed",
            "Order Failed",
            &format!("Your {} payment failed. Please contact support if needed.", order.utility_slug),
            serde_json::json!({ "utility_slug": order.utility_slug, "reason": reason }),
        );
    }
}

fn start_address_pool_refill(state: HttpState, _config: AppConfig, pool: PgPool) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let Some(client) = state.zcash_client.as_ref().cloned() else {
            tracing::info!("no zcash client — address pool refill disabled");
            return;
        };

        let manager = AddressPoolManager::default_policy();
        let mut ticker = interval(Duration::from_secs(300));

        loop {
            ticker.tick().await;
            jobs.mark_alive("address_pool_refill");

            match manager.run_shielded_refill(&pool, &*client).await {
                Ok(outcome) => {
                    state.observability.metrics().set_address_pool_depth(
                        "shielded",
                        outcome.after,
                    );
                    if matches!(outcome.alert_level, crate::jobs::address_pool::PoolAlertLevel::Critical) {
                        tracing::error!(
                            after = outcome.after,
                            "address pool critically low after refill attempt"
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "address pool refill failed");
                }
            }
        }
    });
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
