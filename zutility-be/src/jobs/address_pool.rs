use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;

use crate::{
    db,
    integrations::zcash::ZcashClient,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressPoolPolicy {
    pub low_water_mark: i64,
    pub refill_batch_size: usize,
    pub critical_low_threshold: i64,
}

impl Default for AddressPoolPolicy {
    fn default() -> Self {
        Self {
            low_water_mark: 500,
            refill_batch_size: 2000,
            critical_low_threshold: 50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolAlertLevel {
    Healthy,
    Low,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressPoolMetrics {
    pub shielded_unused: i64,
    pub transparent_unused: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefillOutcome {
    pub before: i64,
    pub inserted: u64,
    pub after: i64,
    pub alert_level: PoolAlertLevel,
}

#[derive(Debug, Clone)]
pub struct AddressPoolManager {
    policy: AddressPoolPolicy,
}

impl AddressPoolManager {
    pub fn new(policy: AddressPoolPolicy) -> Self {
        Self { policy }
    }

    pub fn default_policy() -> Self {
        Self::new(AddressPoolPolicy::default())
    }

    pub fn policy(&self) -> AddressPoolPolicy {
        self.policy
    }

    pub fn classify_alert(&self, current_depth: i64) -> PoolAlertLevel {
        if current_depth < self.policy.critical_low_threshold {
            PoolAlertLevel::Critical
        } else if current_depth < self.policy.low_water_mark {
            PoolAlertLevel::Low
        } else {
            PoolAlertLevel::Healthy
        }
    }

    pub fn refill_plan(&self, current_depth: i64) -> Option<usize> {
        if current_depth < self.policy.low_water_mark {
            Some(self.policy.refill_batch_size)
        } else {
            None
        }
    }

    pub async fn metrics(&self, pool: &PgPool, network: &str) -> Result<AddressPoolMetrics> {
        let depths = db::load_address_pool_depths(pool, network).await?;
        let shielded_unused = depths
            .iter()
            .find(|item| item.address_type == "shielded")
            .map(|item| item.unused_count)
            .unwrap_or(0);
        let transparent_unused = depths
            .iter()
            .find(|item| item.address_type == "transparent")
            .map(|item| item.unused_count)
            .unwrap_or(0);

        Ok(AddressPoolMetrics {
            shielded_unused,
            transparent_unused,
        })
    }

    pub async fn prefill_on_deploy(
        &self,
        pool: &PgPool,
        zcash: &Arc<dyn ZcashClient>,
        network: &str,
        shielded_count: usize,
    ) -> Result<u64> {
        if shielded_count == 0 {
            return Ok(0);
        }

        let mut addresses = Vec::with_capacity(shielded_count);
        for _ in 0..shielded_count {
            let address = zcash.generate_shielded_address().await?;
            addresses.push(address);
        }
        db::insert_deposit_addresses(pool, "shielded", network, &addresses).await
    }

    pub async fn run_shielded_refill(
        &self,
        pool: &PgPool,
        zcash: &dyn ZcashClient,
        network: &str,
    ) -> Result<RefillOutcome> {
        let before = db::count_unused_deposit_addresses(pool, "shielded", network).await?;
        let planned = self.refill_plan(before).unwrap_or(0);

        let inserted = if planned > 0 {
            let mut addresses = Vec::with_capacity(planned);
            for _ in 0..planned {
                match zcash.generate_shielded_address().await {
                    Ok(address) => addresses.push(address),
                    Err(error) => {
                        tracing::warn!(error = %error, "shielded address generation failed during refill");
                        break;
                    }
                }
            }
            if !addresses.is_empty() {
                db::insert_deposit_addresses(pool, "shielded", network, &addresses).await?
            } else {
                0
            }
        } else {
            0
        };

        let after = db::count_unused_deposit_addresses(pool, "shielded", network).await?;
        let alert_level = self.classify_alert(after);

        match alert_level {
            PoolAlertLevel::Critical => {
                tracing::error!(
                    before,
                    after,
                    inserted,
                    "shielded address pool is critically low"
                )
            }
            PoolAlertLevel::Low => {
                tracing::warn!(
                    before,
                    after,
                    inserted,
                    "shielded address pool is below low-water mark"
                )
            }
            PoolAlertLevel::Healthy => {
                tracing::info!(
                    before,
                    after,
                    inserted,
                    "shielded address pool refill completed"
                )
            }
        }

        Ok(RefillOutcome {
            before,
            inserted,
            after,
            alert_level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AddressPoolManager, AddressPoolPolicy, PoolAlertLevel};

    #[test]
    fn classifies_alert_levels() {
        let manager = AddressPoolManager::new(AddressPoolPolicy {
            low_water_mark: 500,
            refill_batch_size: 2000,
            critical_low_threshold: 50,
        });

        assert_eq!(manager.classify_alert(700), PoolAlertLevel::Healthy);
        assert_eq!(manager.classify_alert(120), PoolAlertLevel::Low);
        assert_eq!(manager.classify_alert(30), PoolAlertLevel::Critical);
    }

    #[test]
    fn plans_refill_when_below_low_water_mark() {
        let manager = AddressPoolManager::new(AddressPoolPolicy {
            low_water_mark: 500,
            refill_batch_size: 2000,
            critical_low_threshold: 50,
        });

        assert_eq!(manager.refill_plan(499), Some(2000));
        assert_eq!(manager.refill_plan(500), None);
        assert_eq!(manager.refill_plan(900), None);
    }
}
