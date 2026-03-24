use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Clone)]
pub struct ObservabilityState {
    metrics: Arc<MetricsRegistry>,
    jobs: Arc<JobLivenessRegistry>,
}

#[derive(Debug)]
pub struct MetricsRegistry {
    order_creations: std::sync::atomic::AtomicU64,
    provider_error_count: std::sync::atomic::AtomicU64,
    provider_latency_total_ms: std::sync::atomic::AtomicU64,
    provider_latency_samples: std::sync::atomic::AtomicU64,
    ws_active_connections: std::sync::atomic::AtomicU64,
    zcash_sync_lag_blocks: std::sync::atomic::AtomicU64,
    failed_orders: std::sync::atomic::AtomicU64,
    flagged_orders: std::sync::atomic::AtomicU64,
    state_transitions: std::sync::RwLock<HashMap<String, u64>>,
    address_pool_depth: std::sync::RwLock<HashMap<String, i64>>,
}

#[derive(Debug)]
pub struct JobLivenessRegistry {
    heartbeats: std::sync::RwLock<HashMap<String, DateTime<Utc>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeStatus {
    pub healthy: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessReport {
    pub db: ProbeStatus,
    pub zcash: ProbeStatus,
    pub rate_cache: ProbeStatus,
    pub jobs: ProbeStatus,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertState {
    pub code: String,
    pub active: bool,
    pub detail: String,
}

impl ObservabilityState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(MetricsRegistry::new()),
            jobs: Arc::new(JobLivenessRegistry::new()),
        }
    }

    pub fn metrics(&self) -> Arc<MetricsRegistry> {
        Arc::clone(&self.metrics)
    }

    pub fn jobs(&self) -> Arc<JobLivenessRegistry> {
        Arc::clone(&self.jobs)
    }

    pub async fn evaluate_alerts(
        &self,
        readiness: &ReadinessReport,
        rate_last_updated: DateTime<Utc>,
    ) -> Vec<AlertState> {
        let provider_error_count = self.metrics.provider_error_count();
        let shielded_depth = self
            .metrics
            .address_pool_depth_for("shielded")
            .await
            .unwrap_or(0);
        let high_failed_flagged = self.metrics.failed_orders() + self.metrics.flagged_orders();
        let rate_age = Utc::now() - rate_last_updated;

        vec![
            AlertState {
                code: String::from("provider_error_spike"),
                active: provider_error_count >= 10,
                detail: format!("provider errors in window: {provider_error_count}"),
            },
            AlertState {
                code: String::from("stale_rates"),
                active: rate_age > Duration::minutes(5),
                detail: format!("rate age seconds: {}", rate_age.num_seconds()),
            },
            AlertState {
                code: String::from("address_pool_critical_low"),
                active: shielded_depth < 50,
                detail: format!("shielded pool depth: {shielded_depth}"),
            },
            AlertState {
                code: String::from("high_failed_or_flagged_orders"),
                active: high_failed_flagged >= 25,
                detail: format!("failed+flagged orders: {high_failed_flagged}"),
            },
            AlertState {
                code: String::from("readiness_degraded"),
                active: !readiness.ready,
                detail: String::from("one or more readiness checks are failing"),
            },
        ]
    }
}

impl Default for ObservabilityState {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            order_creations: std::sync::atomic::AtomicU64::new(0),
            provider_error_count: std::sync::atomic::AtomicU64::new(0),
            provider_latency_total_ms: std::sync::atomic::AtomicU64::new(0),
            provider_latency_samples: std::sync::atomic::AtomicU64::new(0),
            ws_active_connections: std::sync::atomic::AtomicU64::new(0),
            zcash_sync_lag_blocks: std::sync::atomic::AtomicU64::new(0),
            failed_orders: std::sync::atomic::AtomicU64::new(0),
            flagged_orders: std::sync::atomic::AtomicU64::new(0),
            state_transitions: std::sync::RwLock::new(HashMap::new()),
            address_pool_depth: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn increment_order_creations(&self) {
        self.order_creations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_state_transition(&self, from: &str, to: &str) {
        let key = format!("{from}_to_{to}");
        if let Ok(mut map) = self.state_transitions.write() {
            let current = map.get(&key).copied().unwrap_or(0);
            map.insert(key, current.saturating_add(1));
        }
    }

    pub fn record_provider_latency(&self, latency_ms: u64) {
        self.provider_latency_total_ms
            .fetch_add(latency_ms, std::sync::atomic::Ordering::Relaxed);
        self.provider_latency_samples
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_provider_error(&self) {
        self.provider_error_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_ws_active_connections(&self, value: u64) {
        self.ws_active_connections
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_address_pool_depth(&self, address_type: &str, depth: i64) {
        let key = address_type.to_ascii_lowercase();
        if let Ok(mut map) = self.address_pool_depth.write() {
            map.insert(key, depth);
        }
    }

    pub fn set_zcash_sync_lag_blocks(&self, lag_blocks: u64) {
        self.zcash_sync_lag_blocks
            .store(lag_blocks, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_failed_orders(&self, value: u64) {
        self.failed_orders
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_flagged_orders(&self, value: u64) {
        self.flagged_orders
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn provider_error_count(&self) -> u64 {
        self.provider_error_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn failed_orders(&self) -> u64 {
        self.failed_orders
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn flagged_orders(&self) -> u64 {
        self.flagged_orders
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn address_pool_depth_for(&self, address_type: &str) -> Option<i64> {
        self.address_pool_depth
            .read()
            .ok()
            .and_then(|map| map.get(&address_type.to_ascii_lowercase()).copied())
    }

    pub async fn render_prometheus(&self) -> String {
        let transitions = self.state_transitions.read().ok();
        let pools = self.address_pool_depth.read().ok();
        let order_creations = self
            .order_creations
            .load(std::sync::atomic::Ordering::Relaxed);
        let provider_errors = self
            .provider_error_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let provider_latency_total = self
            .provider_latency_total_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let provider_latency_samples = self
            .provider_latency_samples
            .load(std::sync::atomic::Ordering::Relaxed);
        let ws_active = self
            .ws_active_connections
            .load(std::sync::atomic::Ordering::Relaxed);
        let zcash_lag = self
            .zcash_sync_lag_blocks
            .load(std::sync::atomic::Ordering::Relaxed);
        let failed = self.failed_orders();
        let flagged = self.flagged_orders();

        let mut output = String::new();
        output.push_str(&format!("order_creations_total {order_creations}\n"));
        output.push_str(&format!("provider_errors_total {provider_errors}\n"));
        output.push_str(&format!(
            "provider_latency_total_ms {provider_latency_total}\n"
        ));
        output.push_str(&format!(
            "provider_latency_samples_total {provider_latency_samples}\n"
        ));
        output.push_str(&format!("ws_active_connections {ws_active}\n"));
        output.push_str(&format!("zcash_sync_lag_blocks {zcash_lag}\n"));
        output.push_str(&format!("orders_failed_total {failed}\n"));
        output.push_str(&format!("orders_flagged_total {flagged}\n"));

        if let Some(transitions) = transitions {
            for (transition, count) in transitions.iter() {
                output.push_str(&format!(
                    "order_state_transitions_total{{transition=\"{transition}\"}} {count}\n"
                ));
            }
        }

        if let Some(pools) = pools {
            for (address_type, depth) in pools.iter() {
                output.push_str(&format!(
                    "address_pool_depth{{address_type=\"{address_type}\"}} {depth}\n"
                ));
            }
        }

        output
    }
}

impl JobLivenessRegistry {
    pub fn new() -> Self {
        Self {
            heartbeats: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn mark_alive(&self, job_name: &str) {
        if let Ok(mut heartbeats) = self.heartbeats.write() {
            heartbeats.insert(job_name.to_owned(), Utc::now());
        }
    }

    pub fn stale_jobs(&self, max_age_seconds: i64) -> Vec<String> {
        let cutoff = Utc::now() - Duration::seconds(max_age_seconds);
        self.heartbeats
            .read()
            .ok()
            .map(|heartbeats| {
                heartbeats
                    .iter()
                    .filter_map(|(job, last_seen)| {
                        if *last_seen < cutoff {
                            Some(job.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub fn has_any_heartbeat(&self) -> bool {
        self.heartbeats
            .read()
            .map(|heartbeats| !heartbeats.is_empty())
            .unwrap_or(false)
    }
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .json()
        .with_target(false)
        .with_current_span(true)
        .with_span_list(true)
        .with_env_filter(filter)
        .init();
}

#[cfg(test)]
mod tests {
    use super::ObservabilityState;

    #[tokio::test]
    async fn renders_metrics_payload_and_alerts() {
        let state = ObservabilityState::new();
        state.metrics().increment_order_creations();
        state.metrics().set_address_pool_depth("shielded", 20);
        state.metrics().set_failed_orders(10);
        state.metrics().set_flagged_orders(20);

        let metrics = state.metrics().render_prometheus().await;
        assert!(metrics.contains("order_creations_total"));
        assert!(metrics.contains("address_pool_depth"));

        let readiness = super::ReadinessReport {
            db: super::ProbeStatus {
                healthy: true,
                detail: String::from("ok"),
            },
            zcash: super::ProbeStatus {
                healthy: true,
                detail: String::from("ok"),
            },
            rate_cache: super::ProbeStatus {
                healthy: false,
                detail: String::from("stale"),
            },
            jobs: super::ProbeStatus {
                healthy: true,
                detail: String::from("ok"),
            },
            ready: false,
        };

        let alerts = state
            .evaluate_alerts(
                &readiness,
                chrono::Utc::now() - chrono::Duration::minutes(10),
            )
            .await;
        assert!(
            alerts
                .iter()
                .any(|alert| alert.code == "stale_rates" && alert.active)
        );
        assert!(
            alerts
                .iter()
                .any(|alert| alert.code == "address_pool_critical_low" && alert.active)
        );
    }
}
