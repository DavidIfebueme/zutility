use std::{cmp::Ordering, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::Value;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentRate {
    pub zec_ngn: Decimal,
    pub zec_usd: Decimal,
    pub usd_ngn: Decimal,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RateSnapshot {
    pub zec_ngn: Decimal,
    pub zec_usd: Decimal,
    pub usd_ngn: Decimal,
    pub coingecko_zec_ngn: Option<Decimal>,
    pub binance_zec_usd: Option<Decimal>,
    pub kraken_zec_usd: Option<Decimal>,
    pub coinbase_zec_usd: Option<Decimal>,
    pub sources_used: Vec<String>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateAlertLevel {
    Normal,
    DriftWarn,
    DriftHeld,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RateRefreshResult {
    pub current: CurrentRate,
    pub snapshot: RateSnapshot,
    pub alert: RateAlertLevel,
}

#[derive(Debug, Clone)]
pub struct RateOracle {
    client: Client,
    timeout: Duration,
    drift_warn_basis_points: u32,
    drift_hold_basis_points: u32,
    minimum_sources: usize,
}

pub type SharedRateCache = Arc<RwLock<CurrentRate>>;

#[derive(Debug, Clone, Default)]
struct SourceSamples {
    coingecko_zec_ngn: Option<Decimal>,
    zec_usd_samples: Vec<(String, Decimal)>,
    usd_ngn: Option<Decimal>,
}

impl RateOracle {
    pub fn new(timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build rate oracle http client")?;
        Ok(Self {
            client,
            timeout,
            drift_warn_basis_points: 500,
            drift_hold_basis_points: 1000,
            minimum_sources: 2,
        })
    }

    pub async fn refresh(&self, previous: Option<&CurrentRate>) -> Result<RateRefreshResult> {
        let samples = self.fetch_samples().await?;
        let usd_ngn = samples
            .usd_ngn
            .context("usd/ngn source unavailable for refresh")?;
        self.refresh_from_samples(
            samples.zec_usd_samples,
            usd_ngn,
            samples.coingecko_zec_ngn,
            previous,
        )
    }

    pub fn refresh_from_samples(
        &self,
        samples: Vec<(String, Decimal)>,
        usd_ngn: Decimal,
        coingecko_zec_ngn: Option<Decimal>,
        previous: Option<&CurrentRate>,
    ) -> Result<RateRefreshResult> {
        if samples.len() < self.minimum_sources {
            anyhow::bail!("insufficient zec/usd sources for median");
        }

        let mut zec_usd_values = samples.iter().map(|(_, value)| *value).collect::<Vec<_>>();
        let zec_usd =
            decimal_median(&mut zec_usd_values).context("failed to compute median zec/usd rate")?;
        let zec_ngn = (zec_usd * usd_ngn).round_dp(4);
        let fetched_at = Utc::now();

        let current = CurrentRate {
            zec_ngn,
            zec_usd: zec_usd.round_dp(4),
            usd_ngn: usd_ngn.round_dp(4),
            updated_at: fetched_at,
        };

        let alert = match previous {
            Some(previous_rate) => compute_drift_alert(
                previous_rate.zec_ngn,
                current.zec_ngn,
                self.drift_warn_basis_points,
                self.drift_hold_basis_points,
            ),
            None => RateAlertLevel::Normal,
        };

        let effective_current = if matches!(alert, RateAlertLevel::DriftHeld) {
            CurrentRate {
                zec_ngn: previous
                    .map(|value| value.zec_ngn)
                    .unwrap_or(current.zec_ngn),
                zec_usd: previous
                    .map(|value| value.zec_usd)
                    .unwrap_or(current.zec_usd),
                usd_ngn: previous
                    .map(|value| value.usd_ngn)
                    .unwrap_or(current.usd_ngn),
                updated_at: fetched_at,
            }
        } else {
            current
        };

        let snapshot = RateSnapshot {
            zec_ngn,
            zec_usd: zec_usd.round_dp(4),
            usd_ngn: usd_ngn.round_dp(4),
            coingecko_zec_ngn,
            binance_zec_usd: sample_value(&samples, "binance"),
            kraken_zec_usd: sample_value(&samples, "kraken"),
            coinbase_zec_usd: sample_value(&samples, "coinbase"),
            sources_used: samples.into_iter().map(|(name, _)| name).collect(),
            fetched_at,
        };

        Ok(RateRefreshResult {
            current: effective_current,
            snapshot,
            alert,
        })
    }

    async fn fetch_samples(&self) -> Result<SourceSamples> {
        let coingecko = self.fetch_coingecko_prices().await;
        let binance = self.fetch_binance_zec_usd().await;
        let kraken = self.fetch_kraken_zec_usd().await;
        let coinbase = self.fetch_coinbase_zec_usd().await;
        let usd_ngn = self.fetch_usd_ngn().await;

        let mut samples = SourceSamples::default();

        if let Ok((zec_usd, zec_ngn)) = coingecko {
            samples
                .zec_usd_samples
                .push((String::from("coingecko"), zec_usd));
            samples.coingecko_zec_ngn = Some(zec_ngn);
        }
        if let Ok(value) = binance {
            samples
                .zec_usd_samples
                .push((String::from("binance"), value));
        }
        if let Ok(value) = kraken {
            samples
                .zec_usd_samples
                .push((String::from("kraken"), value));
        }
        if let Ok(value) = coinbase {
            samples
                .zec_usd_samples
                .push((String::from("coinbase"), value));
        }
        if let Ok(value) = usd_ngn {
            samples.usd_ngn = Some(value);
        }

        Ok(samples)
    }

    async fn fetch_coingecko_prices(&self) -> Result<(Decimal, Decimal)> {
        let response = self
            .client
            .get("https://api.coingecko.com/api/v3/simple/price?ids=zcash&vs_currencies=usd,ngn")
            .timeout(self.timeout)
            .send()
            .await
            .context("coingecko request failed")?
            .json::<Value>()
            .await
            .context("coingecko response decode failed")?;

        let zec = response
            .get("zcash")
            .context("coingecko zcash payload missing")?;
        let zec_usd = parse_decimal(zec.get("usd").context("coingecko zec usd missing")?)?;
        let zec_ngn = parse_decimal(zec.get("ngn").context("coingecko zec ngn missing")?)?;
        Ok((zec_usd, zec_ngn))
    }

    async fn fetch_binance_zec_usd(&self) -> Result<Decimal> {
        let response = self
            .client
            .get("https://api.binance.com/api/v3/ticker/price?symbol=ZECUSDT")
            .timeout(self.timeout)
            .send()
            .await
            .context("binance request failed")?
            .json::<Value>()
            .await
            .context("binance response decode failed")?;
        parse_decimal(response.get("price").context("binance price missing")?)
    }

    async fn fetch_kraken_zec_usd(&self) -> Result<Decimal> {
        let response = self
            .client
            .get("https://api.kraken.com/0/public/Ticker?pair=ZECUSD")
            .timeout(self.timeout)
            .send()
            .await
            .context("kraken request failed")?
            .json::<Value>()
            .await
            .context("kraken response decode failed")?;

        let result = response
            .get("result")
            .and_then(Value::as_object)
            .context("kraken result payload missing")?;
        let pair_payload = result
            .values()
            .next()
            .context("kraken pair payload missing")?;
        let close = pair_payload
            .get("c")
            .and_then(Value::as_array)
            .context("kraken close array missing")?
            .first()
            .context("kraken close price missing")?;
        parse_decimal(close)
    }

    async fn fetch_coinbase_zec_usd(&self) -> Result<Decimal> {
        let response = self
            .client
            .get("https://api.coinbase.com/v2/prices/ZEC-USD/spot")
            .timeout(self.timeout)
            .send()
            .await
            .context("coinbase request failed")?
            .json::<Value>()
            .await
            .context("coinbase response decode failed")?;

        parse_decimal(
            response
                .get("data")
                .and_then(|data| data.get("amount"))
                .context("coinbase amount missing")?,
        )
    }

    async fn fetch_usd_ngn(&self) -> Result<Decimal> {
        let response = self
            .client
            .get("https://open.er-api.com/v6/latest/USD")
            .timeout(self.timeout)
            .send()
            .await
            .context("usd/ngn request failed")?
            .json::<Value>()
            .await
            .context("usd/ngn response decode failed")?;

        parse_decimal(
            response
                .get("rates")
                .and_then(|rates| rates.get("NGN"))
                .context("usd/ngn rate missing")?,
        )
    }
}

pub fn new_shared_rate_cache(initial: CurrentRate) -> SharedRateCache {
    Arc::new(RwLock::new(initial))
}

pub fn default_current_rate() -> CurrentRate {
    CurrentRate {
        zec_ngn: Decimal::new(150_000_0000, 4),
        zec_usd: Decimal::new(100_0000, 4),
        usd_ngn: Decimal::new(1500_0000, 4),
        updated_at: Utc::now(),
    }
}

fn sample_value(samples: &[(String, Decimal)], source: &str) -> Option<Decimal> {
    samples
        .iter()
        .find(|(name, _)| name == source)
        .map(|(_, value)| *value)
}

fn parse_decimal(value: &Value) -> Result<Decimal> {
    if let Some(text) = value.as_str() {
        return text
            .parse::<Decimal>()
            .context("failed to parse decimal from string");
    }
    if let Some(number) = value.as_f64() {
        return Decimal::from_f64_retain(number).context("failed to parse decimal from number");
    }
    anyhow::bail!("unsupported decimal value type")
}

fn decimal_median(values: &mut [Decimal]) -> Option<Decimal> {
    if values.is_empty() {
        return None;
    }

    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some(((values[mid - 1] + values[mid]) / Decimal::new(2, 0)).round_dp(8))
    }
}

fn compute_drift_alert(
    previous_zec_ngn: Decimal,
    new_zec_ngn: Decimal,
    warn_basis_points: u32,
    hold_basis_points: u32,
) -> RateAlertLevel {
    if previous_zec_ngn <= Decimal::ZERO {
        return RateAlertLevel::Normal;
    }

    let delta = (new_zec_ngn - previous_zec_ngn).abs();
    let ratio = (delta / previous_zec_ngn) * Decimal::new(10_000, 0);

    if ratio > Decimal::from(hold_basis_points) {
        RateAlertLevel::DriftHeld
    } else if ratio > Decimal::from(warn_basis_points) {
        RateAlertLevel::DriftWarn
    } else {
        RateAlertLevel::Normal
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rust_decimal::Decimal;

    use super::{CurrentRate, RateAlertLevel, RateOracle, decimal_median};

    #[test]
    fn computes_decimal_median_even_and_odd() {
        let mut odd = vec![Decimal::new(2, 0), Decimal::new(1, 0), Decimal::new(3, 0)];
        assert_eq!(decimal_median(&mut odd), Some(Decimal::new(2, 0)));

        let mut even = vec![
            Decimal::new(10, 0),
            Decimal::new(40, 0),
            Decimal::new(20, 0),
            Decimal::new(30, 0),
        ];
        assert_eq!(decimal_median(&mut even), Some(Decimal::new(25, 0)));
    }

    #[test]
    fn refresh_from_samples_warns_and_holds_on_large_drift() {
        let oracle = RateOracle::new(Duration::from_millis(500))
            .unwrap_or_else(|error| panic!("failed to create oracle: {error}"));

        let previous = CurrentRate {
            zec_ngn: Decimal::new(100_000_0000, 4),
            zec_usd: Decimal::new(100_0000, 4),
            usd_ngn: Decimal::new(1000_0000, 4),
            updated_at: chrono::Utc::now(),
        };

        let warn = oracle
            .refresh_from_samples(
                vec![
                    (String::from("binance"), Decimal::new(106, 0)),
                    (String::from("coinbase"), Decimal::new(107, 0)),
                    (String::from("kraken"), Decimal::new(105, 0)),
                ],
                Decimal::new(1000, 0),
                None,
                Some(&previous),
            )
            .unwrap_or_else(|error| panic!("warn refresh failed: {error}"));
        assert_eq!(warn.alert, RateAlertLevel::DriftWarn);

        let held = oracle
            .refresh_from_samples(
                vec![
                    (String::from("binance"), Decimal::new(120, 0)),
                    (String::from("coinbase"), Decimal::new(122, 0)),
                    (String::from("kraken"), Decimal::new(121, 0)),
                ],
                Decimal::new(1000, 0),
                None,
                Some(&previous),
            )
            .unwrap_or_else(|error| panic!("hold refresh failed: {error}"));
        assert_eq!(held.alert, RateAlertLevel::DriftHeld);
        assert_eq!(held.current.zec_ngn, previous.zec_ngn);
    }
}
