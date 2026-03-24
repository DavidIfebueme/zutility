use anyhow::Result;
use sqlx::PgPool;

use crate::{
    db::{self, PersistRateSnapshotInput},
    integrations::rates::{RateAlertLevel, RateOracle, SharedRateCache},
};

#[derive(Debug, Clone)]
pub struct RateRefresher {
    oracle: RateOracle,
    cache: SharedRateCache,
}

impl RateRefresher {
    pub fn new(oracle: RateOracle, cache: SharedRateCache) -> Self {
        Self { oracle, cache }
    }

    pub async fn refresh_once(&self, pool: &PgPool) -> Result<()> {
        let previous = self.cache.read().await.clone();
        let refreshed = self.oracle.refresh(Some(&previous)).await?;

        let snapshot_input = PersistRateSnapshotInput {
            zec_ngn: refreshed.snapshot.zec_ngn,
            zec_usd: refreshed.snapshot.zec_usd,
            usd_ngn: refreshed.snapshot.usd_ngn,
            coingecko_zec_ngn: refreshed.snapshot.coingecko_zec_ngn,
            binance_zec_usd: refreshed.snapshot.binance_zec_usd,
            kraken_zec_usd: refreshed.snapshot.kraken_zec_usd,
            coinbase_zec_usd: refreshed.snapshot.coinbase_zec_usd,
            sources_used: refreshed.snapshot.sources_used.clone(),
            fetched_at: refreshed.snapshot.fetched_at,
        };
        let _ = db::persist_rate_snapshot(pool, &snapshot_input).await?;

        {
            let mut cache = self.cache.write().await;
            *cache = refreshed.current;
        }

        match refreshed.alert {
            RateAlertLevel::Normal => tracing::info!("rate refresh completed"),
            RateAlertLevel::DriftWarn => tracing::warn!("rate drift exceeded warning threshold"),
            RateAlertLevel::DriftHeld => {
                tracing::error!("rate drift exceeded hold threshold; previous rate held")
            }
        }

        Ok(())
    }
}
