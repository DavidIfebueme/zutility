use std::time::Duration;

use rust_decimal::Decimal;
use zutility_be::integrations::rates::{CurrentRate, RateAlertLevel, RateOracle};

#[test]
fn refresh_from_samples_computes_median_and_ngn_conversion() {
    let oracle = RateOracle::new(Duration::from_millis(300))
        .unwrap_or_else(|error| panic!("failed to construct rate oracle: {error}"));

    let result = oracle
        .refresh_from_samples(
            vec![
                (String::from("binance"), Decimal::new(10000, 2)),
                (String::from("kraken"), Decimal::new(10100, 2)),
                (String::from("coinbase"), Decimal::new(9900, 2)),
            ],
            Decimal::new(150000, 2),
            Some(Decimal::new(15000000, 2)),
            None,
        )
        .unwrap_or_else(|error| panic!("rate refresh failed: {error}"));

    assert_eq!(result.alert, RateAlertLevel::Normal);
    assert_eq!(result.current.zec_usd, Decimal::new(10000, 2));
    assert_eq!(result.current.usd_ngn, Decimal::new(150000, 2));
    assert_eq!(result.current.zec_ngn, Decimal::new(15000000, 2));
    assert_eq!(result.snapshot.sources_used.len(), 3);
}

#[test]
fn refresh_holds_previous_rate_when_drift_exceeds_10_percent() {
    let oracle = RateOracle::new(Duration::from_millis(300))
        .unwrap_or_else(|error| panic!("failed to construct rate oracle: {error}"));

    let previous = CurrentRate {
        zec_ngn: Decimal::new(15000000, 2),
        zec_usd: Decimal::new(10000, 2),
        usd_ngn: Decimal::new(150000, 2),
        updated_at: chrono::Utc::now(),
    };

    let result = oracle
        .refresh_from_samples(
            vec![
                (String::from("binance"), Decimal::new(12000, 2)),
                (String::from("kraken"), Decimal::new(12200, 2)),
                (String::from("coinbase"), Decimal::new(12100, 2)),
            ],
            Decimal::new(150000, 2),
            None,
            Some(&previous),
        )
        .unwrap_or_else(|error| panic!("rate refresh failed: {error}"));

    assert_eq!(result.alert, RateAlertLevel::DriftHeld);
    assert_eq!(result.current.zec_ngn, previous.zec_ngn);
}
