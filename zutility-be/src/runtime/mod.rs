use std::collections::HashMap;
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
    domain::{
        gift::{GiftStatus, HeldGiftOutcome, held_gift_outcome},
        order::{OrderStatus, OrderStatusTransition, ThresholdPolicy},
    },
    http::handlers::HttpState,
    integrations::{
        dtone::PROVIDER_NAME as DTONE_PROVIDER_NAME,
        provider_dispatcher::ProviderDispatcher,
        rates::RateOracle,
        remita::RemitaClient,
        utility_provider::{
            ProviderContext, ProviderErrorKind, ProviderTxnStatus, UtilityPurchaseRequest,
        },
        vtpass::VtpassClient,
        zcash::{ReceivedNote, group_notes_by_address},
    },
    jobs::{
        address_pool::AddressPoolManager, catalogue_refresher::CatalogueRefresher,
        rate_refresher::RateRefresher,
    },
    ws::WsOrderEvent,
};

pub const DTONE_DISPATCH_TIMEOUT_FLAG: &str = "dtone_dispatch_timeout";
pub const DTONE_RESOLUTION_TIMEOUT_HOURS: i64 = 24;

pub fn start_background_workers(state: HttpState, config: AppConfig, pool: PgPool) {
    start_http_heartbeat(&state);
    start_rate_refresher(state.clone(), config.clone(), pool.clone());
    start_catalogue_refresher(state.clone(), config.clone(), pool.clone());
    start_order_orchestrator(state.clone(), config.clone(), pool.clone());
    start_address_pool_refill(state, config, pool);
}

pub fn start_http_heartbeat(state: &HttpState) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            jobs.mark_alive("http_server");
        }
    });
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

fn start_catalogue_refresher(state: HttpState, config: AppConfig, pool: PgPool) {
    let jobs = state.observability.jobs();
    tokio::spawn(async move {
        let client = match crate::integrations::dtone::DtoneClient::from_config(&config) {
            Ok(client) => client,
            Err(error) => {
                tracing::info!(error = %error, "no dtone client — catalogue refresher disabled");
                return;
            }
        };

        let refresher = CatalogueRefresher::new(std::sync::Arc::new(client));
        let seconds = config.catalogue_refresh_seconds.max(60);
        let mut ticker = interval(Duration::from_secs(seconds));

        loop {
            ticker.tick().await;
            jobs.mark_alive("catalogue_refresher");

            match refresher.refresh_once(&pool).await {
                Ok(updated) => {
                    tracing::info!(updated, "catalogue refresh iteration finished");
                }
                Err(error) => {
                    tracing::warn!(error = %error, "catalogue refresh iteration failed");
                }
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

        let remita_collections =
            match crate::integrations::remita_collections::RemitaCollectionsClient::from_config(
                &config,
            ) {
                Ok(client) => {
                    tracing::info!("remita collections client initialized successfully");
                    Some(client)
                }
                Err(error) => {
                    tracing::warn!(error = %error, "remita collections client not configured — school fees will be unavailable");
                    None
                }
            };

        let dtone = match crate::integrations::dtone::DtoneClient::from_config(&config) {
            Ok(client) => {
                tracing::info!("dtone client initialized successfully");
                Some(client)
            }
            Err(error) => {
                tracing::warn!(error = %error, "dtone client not configured — multi-country utilities will be unavailable");
                None
            }
        };

        let dispatcher =
            ProviderDispatcher::new(vtpass, remita, remita_collections, inlomax, dtone)
                .with_metrics(state.observability.metrics());

        match db::list_configured_product_providers(&pool).await {
            Ok(rows) => dispatcher.warn_unconfigured_providers(&rows),
            Err(error) => {
                tracing::warn!(error = %error, "could not check product provider configuration");
            }
        }

        let mut ticker = interval(Duration::from_secs(25));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut consecutive_sync_failures: u32 = 0;
        loop {
            ticker.tick().await;
            jobs.mark_alive("confirmation_watcher");
            jobs.mark_alive("utility_dispatcher");
            jobs.mark_alive("order_timeout_reaper");
            jobs.mark_alive("gift_claim_expiry");

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
                    let since_timestamp = orders
                        .iter()
                        .map(|order| order.created_at.timestamp().saturating_sub(600))
                        .min()
                        .unwrap_or_else(|| Utc::now().timestamp());
                    let (payments, scan_ok) = if let Some(client) = state.zcash_client.as_ref() {
                        match client
                            .scan_payments(u32::try_from(since_timestamp).unwrap_or(0))
                            .await
                        {
                            Ok(notes) => (group_notes_by_address(notes), true),
                            Err(error) => {
                                tracing::warn!(error = %error, "wallet payment scan failed");
                                (HashMap::new(), false)
                            }
                        }
                    } else {
                        (HashMap::new(), false)
                    };

                    for order in orders {
                        if let Err(error) =
                            process_order(&state, &dispatcher, &pool, &order, &payments, scan_ok)
                                .await
                        {
                            tracing::warn!(order_id = %order.id, error = %error, "order processing iteration failed");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "failed to list active orders");
                }
            }

            if let Err(error) = expire_claim_windows(&state).await {
                tracing::warn!(error = %error, "gift claim window expiry failed");
            }

            if let Some(client) = state.zcash_client.as_ref()
                && let Err(error) = client.save_wallet().await
            {
                tracing::warn!(error = %error, "zingolib wallet save failed");
            }
        }
    });
}

fn expiry_transition(
    status: OrderStatus,
    expires_at: chrono::DateTime<Utc>,
    now: chrono::DateTime<Utc>,
) -> Result<Option<OrderStatusTransition>> {
    let grace = chrono::Duration::minutes(i64::from(
        ThresholdPolicy::default().late_payment_grace_minutes,
    ));
    let deadline = match status {
        OrderStatus::AwaitingPayment => expires_at,
        OrderStatus::PaymentDetected => expires_at + grace,
        _ => return Ok(None),
    };

    if deadline > now {
        return Ok(None);
    }

    Ok(Some(OrderStatusTransition::new(
        status,
        OrderStatus::Expired,
    )?))
}

async fn process_order(
    state: &HttpState,
    dispatcher: &ProviderDispatcher,
    pool: &PgPool,
    order: &db::OrderRow,
    payments: &HashMap<String, Vec<ReceivedNote>>,
    scan_ok: bool,
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

    if let Some(transition) = expiry_transition(order_status, order.expires_at, now)? {
        let expired = db::update_order_status_cas(
            pool,
            order.id,
            transition.from.as_db(),
            transition.to.as_db(),
            "order_expired",
            serde_json::json!({ "expired_at": now }),
        )
        .await?;
        if expired {
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
        }
        return Ok(());
    }

    if matches!(
        order_status,
        OrderStatus::AwaitingPayment | OrderStatus::PaymentDetected
    ) && state.zcash_client.is_some()
    {
        detect_payment_and_progress(state, pool, order, payments, scan_ok).await?;
    }

    let refreshed = db::get_order_by_id(pool, order.id).await?;
    let Some(updated) = refreshed else {
        return Ok(());
    };
    let updated_status = updated.status()?;

    if updated_status == OrderStatus::AwaitingClaim {
        resolve_held_gift(state, &updated).await?;
        return Ok(());
    }

    if updated_status == OrderStatus::FlaggedForReview {
        if updated.flag_reason.as_deref() == Some(DTONE_DISPATCH_TIMEOUT_FLAG) {
            requery_utility_dispatch(state, dispatcher, pool, &updated).await?;
        }
        return Ok(());
    }

    if updated_status == OrderStatus::PaymentConfirmed {
        dispatch_utility(state, dispatcher, pool, &updated).await?;
    } else if updated_status == OrderStatus::UtilityDispatching {
        requery_utility_dispatch(state, dispatcher, pool, &updated).await?;
    }

    Ok(())
}

async fn expire_claim_windows(state: &HttpState) -> Result<()> {
    let now = Utc::now();
    for gift in db::list_expired_open_gifts(&state.pool, now).await? {
        if let Err(error) = expire_one_gift(state, gift).await {
            tracing::warn!(error = %error, "gift expiry iteration failed");
        }
    }

    Ok(())
}

async fn expire_one_gift(state: &HttpState, gift: db::GiftRow) -> Result<()> {
    let pool = &state.pool;
    let Some(order_id) = gift.order_id else {
        let _ = db::expire_gift_cas(pool, gift.id).await?;
        return Ok(());
    };
    let Some(order) = db::get_order_by_id(pool, order_id).await? else {
        let _ = db::expire_gift_cas(pool, gift.id).await?;
        return Ok(());
    };

    let status = order.status().ok();

    if matches!(
        status,
        Some(OrderStatus::AwaitingPayment)
            | Some(OrderStatus::PaymentDetected)
            | Some(OrderStatus::PaymentConfirmed)
    ) {
        return Ok(());
    }

    if !db::expire_gift_cas(pool, gift.id).await? {
        return Ok(());
    }

    if status == Some(OrderStatus::AwaitingClaim) {
        deliver_expired_gift(state, &gift, &order).await?;
    }

    Ok(())
}

async fn resolve_held_gift(state: &HttpState, order: &db::OrderRow) -> Result<()> {
    let pool = &state.pool;
    let Some(gift) = db::find_gift_by_order_id(pool, order.id).await? else {
        return Ok(());
    };
    let Some(status) = gift.status() else {
        return Ok(());
    };

    match held_gift_outcome(status, gift.claim_expires_at, Utc::now()) {
        HeldGiftOutcome::KeepHeld => {}
        HeldGiftOutcome::ReleaseToDispatch => {
            db::update_order_status_cas(
                pool,
                order.id,
                "awaiting_claim",
                "payment_confirmed",
                "gift_claim_released",
                serde_json::json!({ "gift_id": gift.id }),
            )
            .await?;
        }
        HeldGiftOutcome::ResolveExpired => {
            if status == GiftStatus::Open && !db::expire_gift_cas(pool, gift.id).await? {
                return Ok(());
            }
            deliver_expired_gift(state, &gift, order).await?;
        }
        HeldGiftOutcome::FlagForReview => {
            let flagged = db::update_order_status_cas(
                pool,
                order.id,
                "awaiting_claim",
                "flagged_for_review",
                "gift_cancelled_while_held",
                serde_json::json!({ "gift_id": gift.id }),
            )
            .await?;
            if flagged {
                broadcast_flagged(state, order.id, "gift_cancelled_while_held").await;
            }
        }
    }

    Ok(())
}

async fn deliver_expired_gift(
    state: &HttpState,
    gift: &db::GiftRow,
    order: &db::OrderRow,
) -> Result<()> {
    let pool = &state.pool;
    let fallback = gift
        .fallback_service_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match fallback {
        Some(service_ref) => {
            db::update_order_service_ref(pool, order.id, service_ref, None).await?;
            db::update_order_status_cas(
                pool,
                order.id,
                "awaiting_claim",
                "payment_confirmed",
                "gift_expired_fallback",
                serde_json::json!({ "gift_id": gift.id }),
            )
            .await?;
            notify_order(
                pool,
                order,
                "gift_expired_fallback",
                "Gift Expired",
                "Nobody claimed the gift in time, so it was delivered to your backup account.",
                serde_json::json!({ "gift_id": gift.id, "utility_slug": gift.utility_slug }),
            );
        }
        None => {
            let flagged = db::update_order_status_cas(
                pool,
                order.id,
                "awaiting_claim",
                "flagged_for_review",
                "gift_expired_unclaimed",
                serde_json::json!({ "gift_id": gift.id }),
            )
            .await?;
            if flagged {
                broadcast_flagged(state, order.id, "gift_expired_unclaimed").await;
                notify_order(
                    pool,
                    order,
                    "gift_expired_unclaimed",
                    "Gift Needs Attention",
                    "Nobody claimed the gift and no backup account was set. Support will follow up.",
                    serde_json::json!({ "gift_id": gift.id, "utility_slug": order.utility_slug }),
                );
            }
        }
    }

    Ok(())
}

fn primary_txid(notes: &[ReceivedNote]) -> Option<String> {
    notes
        .iter()
        .max_by(|a, b| {
            (a.amount, a.confirmations, &a.txid).cmp(&(b.amount, b.confirmations, &b.txid))
        })
        .map(|note| note.txid.clone())
}

fn note_ordinals(notes: &[ReceivedNote]) -> Vec<i32> {
    let mut counts: HashMap<(String, String, String), i32> = HashMap::new();

    notes
        .iter()
        .map(|note| {
            let key = (
                note.txid.clone(),
                note.address.clone(),
                note.amount.to_string(),
            );
            let entry = counts.entry(key).or_insert(-1);
            *entry += 1;
            *entry
        })
        .collect()
}

async fn detect_payment_and_progress(
    state: &HttpState,
    pool: &PgPool,
    order: &db::OrderRow,
    payments: &HashMap<String, Vec<ReceivedNote>>,
    scan_ok: bool,
) -> Result<()> {
    let notes = payments
        .get(&order.deposit_address)
        .cloned()
        .unwrap_or_default();
    let total_received = notes
        .iter()
        .fold(Decimal::ZERO, |acc, note| acc + note.amount);
    let confirmations = u16::try_from(
        notes
            .iter()
            .map(|note| note.confirmations)
            .min()
            .unwrap_or(0),
    )
    .unwrap_or(u16::MAX);

    for (note, note_index) in notes.iter().zip(note_ordinals(&notes)) {
        if let Err(error) = db::record_payment(
            pool,
            order.id,
            &note.txid,
            &note.address,
            note.amount,
            note_index,
            i32::try_from(note.confirmations).unwrap_or(i32::MAX),
        )
        .await
        {
            tracing::warn!(
                order_id = %order.id,
                txid = %note.txid,
                error = %error,
                "failed to record payment"
            );
        }
    }

    let mut txids: Vec<String> = notes.iter().map(|note| note.txid.clone()).collect();
    txids.sort();
    txids.dedup();

    if scan_ok && let Err(error) = db::mark_missing_payments(pool, order.id, &txids).await {
        tracing::warn!(order_id = %order.id, error = %error, "failed to reconcile payments");
    }

    if total_received <= Decimal::ZERO {
        return Ok(());
    }

    let confirmations_i32 = i32::from(confirmations);
    let required_confs = order.required_confs;
    let primary_txid = primary_txid(&notes);

    db::update_order_payment(
        pool,
        order.id,
        confirmations_i32,
        total_received,
        primary_txid.as_deref(),
    )
    .await?;

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
                "txids": &txids,
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
                    "txids": &txids,
                }),
            )
            .await?;
            events.push(WsOrderEvent::Flagged {
                reason: String::from("underpaid_flagged"),
            });
            notify_order(
                pool,
                order,
                "order_flagged",
                "Order Under Review",
                &format!(
                    "Your {} payment is being reviewed due to an underpayment.",
                    order.utility_slug
                ),
                serde_json::json!({ "utility_slug": order.utility_slug }),
            );
        } else {
            let gift = db::find_gift_by_order_id(pool, order.id).await?;
            let hold_for_claim = matches!(
                gift.as_ref().and_then(|row| row.status()),
                Some(crate::domain::gift::GiftStatus::Open)
            );

            if hold_for_claim && let Some(ref gift) = gift {
                let claim_expires_at =
                    Utc::now() + chrono::Duration::hours(i64::from(gift.claim_window_hours));
                db::set_gift_claim_expiry(pool, gift.id, claim_expires_at).await?;
            }

            let (next_status, event_name) = if hold_for_claim {
                ("awaiting_claim", "gift_awaiting_claim")
            } else {
                ("payment_confirmed", "payment_confirmed")
            };

            db::update_order_status_cas(
                pool,
                order.id,
                "payment_detected",
                next_status,
                event_name,
                serde_json::json!({
                    "confirmations": confirmations,
                    "gift_hold": hold_for_claim,
                    "txids": &txids,
                }),
            )
            .await?;

            if hold_for_claim {
                events.push(WsOrderEvent::AwaitingClaim);
                notify_order(
                    pool,
                    order,
                    "gift_awaiting_claim",
                    "Gift Ready to Claim",
                    &format!(
                        "Your {} gift is paid. Share the link so it can be claimed.",
                        order.utility_slug
                    ),
                    serde_json::json!({ "utility_slug": order.utility_slug }),
                );
            } else {
                events.push(WsOrderEvent::PaymentConfirmed { confirmations });
                notify_order(
                    pool,
                    order,
                    "payment_confirmed",
                    "Payment Confirmed",
                    &format!(
                        "Your payment for {} has been confirmed. Processing your order...",
                        order.utility_slug
                    ),
                    serde_json::json!({ "utility_slug": order.utility_slug }),
                );
            }
        }
    }

    for event in events {
        let _ = state.ws_hub.broadcast_event(order.id, &event).await;
    }

    Ok(())
}

pub fn purchase_request_for(
    order: &db::OrderRow,
    product: Option<&db::ProductRow>,
    variation: Option<&db::ProductVariationRow>,
    request_id: &str,
) -> Result<UtilityPurchaseRequest> {
    let product_id = variation
        .map(|variation| variation.provider_product_id.clone())
        .or_else(|| product.map(|product| product.provider_product_id.clone()))
        .unwrap_or_default();

    let metadata = match product {
        Some(product) => serde_json::json!({
            "utility_type": order.utility_type,
            "customer_name": order.customer_name,
            "meter_type": order.metadata.get("meter_type"),
            "variation_code": order.variation_code,
            "amount_kind": product.amount_kind,
            "required_fields": product.required_fields,
            "fulfilment_kind": product.fulfilment_kind,
            "country_iso2": product.country_iso2,
        }),
        None => order.metadata.clone(),
    };

    Ok(UtilityPurchaseRequest {
        order_id: order.id,
        request_id: request_id.to_owned(),
        context: ProviderContext {
            utility_type: order.utility_type.clone(),
            service_id: order.utility_slug.clone(),
            billers_code: order.service_ref.clone(),
            variation_code: order.variation_code.clone(),
            meter_type: order
                .metadata
                .get("meter_type")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            phone: Some(order.service_ref.clone()),
        },
        cost_ngn: order.cost()?,
        settlement: order.settlement()?,
        product_id,
        external_id: order.id.to_string(),
        inputs: order.inputs.clone(),
        metadata,
        zec_amount: order.zec_amount,
    })
}

async fn load_order_product(
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<(Option<db::ProductRow>, Option<db::ProductVariationRow>)> {
    let Some(product_id) = order.product_id else {
        return Ok((None, None));
    };

    let product = db::find_product_by_id(pool, product_id).await?;
    let variation = match (&product, order.variation_code.as_deref()) {
        (Some(product), Some(code)) => db::find_product_variation(pool, product.id, code).await?,
        _ => None,
    };

    Ok((product, variation))
}

fn request_id_for(order: &db::OrderRow, provider_name: &str) -> String {
    match provider_name {
        DTONE_PROVIDER_NAME => order.id.to_string(),
        "remita" | "remita_collections" => order.service_ref.clone(),
        "inlomax" => format!("inl-{}", order.id.as_simple()),
        _ => format!("vp-{}", order.id.as_simple()),
    }
}

async fn dispatch_utility(
    state: &HttpState,
    dispatcher: &ProviderDispatcher,
    pool: &PgPool,
    order: &db::OrderRow,
) -> Result<()> {
    let (product, variation) = load_order_product(pool, order).await?;

    let provider_name = match &product {
        Some(product) => product.provider.clone(),
        None => order.provider.clone().unwrap_or_else(|| {
            format!("{:?}", dispatcher.provider_kind_for(&order.utility_type)).to_lowercase()
        }),
    };

    let request_id = request_id_for(order, &provider_name);

    let transitioned =
        db::set_order_dispatching(pool, order.id, &provider_name, Some(&request_id)).await?;
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
        &format!(
            "We're sending your {} payment for processing.",
            order.utility_slug
        ),
        serde_json::json!({ "utility_slug": order.utility_slug }),
    );

    let request = purchase_request_for(order, product.as_ref(), variation.as_ref(), &request_id)?;

    let response = match &product {
        Some(product) => {
            dispatcher
                .pay_for_product(
                    &product.provider,
                    product.failover_provider.as_deref(),
                    &request,
                )
                .await
        }
        None => dispatcher.pay(&order.utility_type, &request).await,
    };

    match response {
        Ok(result) => {
            if result.status != ProviderTxnStatus::Failed
                && let Err(error) = db::mark_provider_accepted(pool, order.id).await
            {
                tracing::warn!(order_id = %order.id, error = %error, "failed to record provider acceptance");
            }

            if !result.provider_reference.is_empty()
                && result.provider_reference != result.provider_request_id
                && let Err(error) = db::store_provider_transaction_id(
                    pool,
                    order.id,
                    &result.provider_reference,
                    Some(&format!("{:?}", result.status).to_lowercase()),
                )
                .await
            {
                tracing::warn!(order_id = %order.id, error = %error, "failed to store provider transaction id");
            }

            if result.status == ProviderTxnStatus::Delivered {
                complete_order(
                    state,
                    pool,
                    order.id,
                    result.token.as_deref(),
                    Some(result.provider_request_id.as_str()),
                )
                .await;
            } else if result.status == ProviderTxnStatus::Failed {
                db::record_failure(
                    pool,
                    order.id,
                    "provider_failed",
                    "the provider reported a failure",
                )
                .await?;
                fail_order(state, pool, order.id, "provider_failed").await;
            } else if !result.provider_request_id.is_empty()
                && let Err(error) =
                    db::store_provider_reference(pool, order.id, &result.provider_request_id).await
            {
                tracing::warn!(order_id = %order.id, error = %error, "failed to store provider reference for requery");
            }
        }
        Err(error)
            if order.utility_type == "school_fees"
                && matches!(error.kind, ProviderErrorKind::Permanent) =>
        {
            let transitioned = db::update_order_status_cas(
                pool,
                order.id,
                "utility_dispatching",
                "flagged_for_review",
                "provider_rejected",
                serde_json::json!({ "reason": error.message }),
            )
            .await?;

            if transitioned {
                tracing::error!(order_id = %order.id, "provider rejected the RRR, flagged for review");
                broadcast_flagged(state, order.id, "provider_rejected").await;
                notify_order(
                    pool,
                    order,
                    "order_flagged",
                    "Order Under Review",
                    "The provider rejected this RRR. Support will follow up.",
                    serde_json::json!({ "utility_slug": order.utility_slug }),
                );
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
    let (product, _variation) = load_order_product(pool, order).await?;
    let provider_name = match &product {
        Some(product) => product.provider.clone(),
        None => order.provider.clone().unwrap_or_else(|| {
            format!("{:?}", dispatcher.provider_kind_for(&order.utility_type)).to_lowercase()
        }),
    };

    let fallback_ref = order.id.to_string();
    let reference = if provider_name == DTONE_PROVIDER_NAME {
        order
            .provider_external_id
            .as_deref()
            .unwrap_or(&fallback_ref)
    } else {
        order.vtpass_request_id.as_deref().unwrap_or(&fallback_ref)
    };

    if order.vtpass_request_id.is_none() && order.provider_external_id.is_none() {
        tracing::warn!(order_id = %order.id, "requery has no stored provider reference, falling back to order ID");
    }

    let accepted = order
        .metadata
        .get("provider_accepted")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let mut current_status = order.status().unwrap_or(OrderStatus::UtilityDispatching);

    if current_status == OrderStatus::UtilityDispatching {
        let started_at = db::dispatch_started_at(pool, order.id)
            .await?
            .unwrap_or_else(|| order.confirmed_at.unwrap_or(order.created_at));

        if Utc::now() - started_at > chrono::Duration::minutes(120) {
            let transitioned = db::update_order_status_cas(
                pool,
                order.id,
                "utility_dispatching",
                "flagged_for_review",
                DTONE_DISPATCH_TIMEOUT_FLAG,
                serde_json::json!({ "reference": reference, "minutes": 120 }),
            )
            .await?;

            if transitioned {
                current_status = OrderStatus::FlaggedForReview;
                tracing::error!(order_id = %order.id, "dispatch stuck for 2 hours, flagged");
                broadcast_flagged(state, order.id, DTONE_DISPATCH_TIMEOUT_FLAG).await;
                notify_order(
                    pool,
                    order,
                    "order_flagged",
                    "Order Under Review",
                    "The provider has not confirmed this order. Support will follow up.",
                    serde_json::json!({ "utility_slug": order.utility_slug }),
                );
            }
        }
    }

    if current_status == OrderStatus::FlaggedForReview
        && provider_name == DTONE_PROVIDER_NAME
        && Utc::now() - order.created_at > chrono::Duration::hours(DTONE_RESOLUTION_TIMEOUT_HOURS)
    {
        let failed = db::update_order_status_cas(
            pool,
            order.id,
            "flagged_for_review",
            "failed",
            "dtone_resolution_timeout",
            serde_json::json!({ "reference": reference, "hours": DTONE_RESOLUTION_TIMEOUT_HOURS }),
        )
        .await?;

        if failed {
            tracing::error!(order_id = %order.id, "dt one order unresolved for 24 hours, failed");
            let _ = state
                .ws_hub
                .broadcast_event(
                    order.id,
                    &WsOrderEvent::Failed {
                        reason: String::from("dtone_resolution_timeout"),
                    },
                )
                .await;
        }
        return Ok(());
    }

    let response = match &product {
        Some(product) => {
            dispatcher
                .requery_for_product(&product.provider, reference)
                .await
        }
        None => dispatcher.requery(&order.utility_type, reference).await,
    };

    match response {
        Ok(result) if result.status == ProviderTxnStatus::Delivered => {
            if order.utility_type == "school_fees" && !accepted {
                let transitioned = db::update_order_status_cas(
                    pool,
                    order.id,
                    "utility_dispatching",
                    "flagged_for_review",
                    "provider_paid_without_acceptance",
                    serde_json::json!({ "reference": reference }),
                )
                .await?;

                if transitioned {
                    tracing::error!(
                        order_id = %order.id,
                        "RRR paid without an accepted process call, flagged"
                    );
                    broadcast_flagged(state, order.id, "provider_paid_without_acceptance").await;
                    notify_order(
                        pool,
                        order,
                        "order_flagged",
                        "Order Under Review",
                        "The RRR was paid without our settlement acknowledgement. Support will follow up.",
                        serde_json::json!({ "utility_slug": order.utility_slug }),
                    );
                }

                return Ok(());
            }

            complete_order(
                state,
                pool,
                order.id,
                result.token.as_deref(),
                Some(result.provider_request_id.as_str()),
            )
            .await;
        }
        Ok(result) if result.status == ProviderTxnStatus::Reversed => {
            let reversed = db::mark_order_reversed(
                pool,
                order.id,
                order.provider_status.as_deref().unwrap_or("80000"),
                None,
            )
            .await?;

            if reversed {
                tracing::error!(order_id = %order.id, "provider reported a reversal, order reversed");
                notify_order(
                    pool,
                    order,
                    "order_reversed",
                    "Order Reversed",
                    &format!(
                        "Your {} payment was reversed by the provider. Support will refund it to {}.",
                        order.utility_slug, order.deposit_address
                    ),
                    serde_json::json!({
                        "utility_slug": order.utility_slug,
                        "refund_address": order.deposit_address,
                    }),
                );
                let _ = state
                    .ws_hub
                    .broadcast_event(order.id, &WsOrderEvent::Reversed)
                    .await;
            }
        }
        Ok(result) if result.status == ProviderTxnStatus::Failed => {
            if matches!(
                current_status,
                OrderStatus::UtilityDispatching | OrderStatus::FlaggedForReview
            ) {
                db::record_failure(
                    pool,
                    order.id,
                    "provider_failed",
                    "the provider reported a failure on requery",
                )
                .await?;
                fail_order(state, pool, order.id, "provider_failed").await;
            }
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(order_id = %order.id, error = %error, "utility requery failed");
        }
    }

    Ok(())
}

async fn broadcast_flagged(state: &HttpState, order_id: Uuid, reason: &str) {
    let _ = state
        .ws_hub
        .broadcast_event(
            order_id,
            &WsOrderEvent::Flagged {
                reason: reason.to_owned(),
            },
        )
        .await;
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
        if let Err(e) =
            db::create_notification(&pool, user_id, Some(order_id), &nt, &t, &b, detail).await
        {
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
    match db::complete_order(pool, order_id, delivery_token, vtpass_request_id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(order_id = %order_id, "complete_order matched no dispatching order");
            return;
        }
        Err(error) => {
            tracing::warn!(order_id = %order_id, error = %error, "failed to complete order in db");
            return;
        }
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
            &format!(
                "Your {} payment was delivered successfully!",
                order.utility_slug
            ),
            serde_json::json!({ "utility_slug": order.utility_slug }),
        );
    }
}

async fn fail_order(state: &HttpState, pool: &PgPool, order_id: Uuid, reason: &str) {
    match db::fail_order(pool, order_id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(order_id = %order_id, "fail_order matched no dispatching order");
            return;
        }
        Err(error) => {
            tracing::warn!(order_id = %order_id, error = %error, "failed to mark order as failed in db");
            return;
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

    if let Ok(Some(order)) = db::get_order_by_id(pool, order_id).await {
        notify_order(
            pool,
            &order,
            "order_failed",
            "Order Failed",
            &format!(
                "Your {} payment failed. Please contact support if needed.",
                order.utility_slug
            ),
            serde_json::json!({ "utility_slug": order.utility_slug, "reason": reason }),
        );
    }
}

fn start_address_pool_refill(state: HttpState, _config: AppConfig, pool: PgPool) {
    let jobs = state.observability.jobs();
    let network = state.zcash_network.as_str().to_owned();
    tokio::spawn(async move {
        let Some(client) = state.zcash_client.as_ref().cloned() else {
            tracing::info!("no zcash client — address pool refill disabled");
            return;
        };

        let manager = AddressPoolManager::default_policy();
        let mut ticker = interval(Duration::from_secs(60));
        let mut ticks_since_refill = 5u32;

        loop {
            ticker.tick().await;
            jobs.mark_alive("address_pool_refill");

            ticks_since_refill = ticks_since_refill.saturating_add(1);
            if ticks_since_refill < 5 {
                continue;
            }
            ticks_since_refill = 0;

            match manager.run_shielded_refill(&pool, &*client, &network).await {
                Ok(outcome) => {
                    state
                        .observability
                        .metrics()
                        .set_address_pool_depth("shielded", outcome.after);
                    if matches!(
                        outcome.alert_level,
                        crate::jobs::address_pool::PoolAlertLevel::Critical
                    ) {
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
    use super::{expiry_transition, note_ordinals, primary_txid};
    use crate::domain::order::{OrderStatus, OrderStatusTransition};
    use crate::integrations::zcash::ReceivedNote;
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn note(txid: &str, amount: i64) -> ReceivedNote {
        ReceivedNote {
            txid: txid.to_owned(),
            address: String::from("utest1example"),
            amount: Decimal::new(amount, 8),
            confirmations: 2,
            memo: None,
        }
    }

    #[test]
    fn underpay_threshold_matches_policy() {
        let expected = Decimal::new(100_000_000, 8);
        let threshold = expected * Decimal::new(995, 3);
        assert_eq!(threshold, Decimal::new(99_500_000, 8));
    }

    #[test]
    fn primary_txid_is_the_largest_note() {
        let notes = vec![
            note("aa", 37_653),
            note("bb", 10_000_000),
            note("cc", 40_000),
        ];

        assert_eq!(primary_txid(&notes).as_deref(), Some("bb"));
    }

    #[test]
    fn primary_txid_is_none_without_notes() {
        assert!(primary_txid(&[]).is_none());
    }

    #[test]
    fn identical_notes_get_distinct_ordinals() {
        let notes = vec![note("aa", 10_000), note("aa", 10_000), note("bb", 20_000)];

        assert_eq!(note_ordinals(&notes), vec![0, 1, 0]);
    }

    #[test]
    fn expiry_covers_awaiting_payment_and_payment_detected() {
        let now = Utc::now();
        let past = now - chrono::Duration::minutes(1);
        let future = now + chrono::Duration::minutes(1);

        assert!(matches!(
            expiry_transition(OrderStatus::AwaitingPayment, past, now),
            Ok(Some(OrderStatusTransition {
                from: OrderStatus::AwaitingPayment,
                to: OrderStatus::Expired
            }))
        ));
        assert!(matches!(
            expiry_transition(OrderStatus::PaymentDetected, past, now),
            Ok(None)
        ));
        assert!(matches!(
            expiry_transition(
                OrderStatus::PaymentDetected,
                now - chrono::Duration::minutes(121),
                now
            ),
            Ok(Some(OrderStatusTransition {
                from: OrderStatus::PaymentDetected,
                to: OrderStatus::Expired
            }))
        ));
        assert!(matches!(
            expiry_transition(OrderStatus::PaymentConfirmed, past, now),
            Ok(None)
        ));
        assert!(matches!(
            expiry_transition(OrderStatus::AwaitingPayment, future, now),
            Ok(None)
        ));
    }
}
