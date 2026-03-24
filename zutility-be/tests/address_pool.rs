use zutility_be::jobs::address_pool::{AddressPoolManager, AddressPoolPolicy, PoolAlertLevel};

#[test]
fn default_policy_matches_step_8_thresholds() {
    let manager = AddressPoolManager::default_policy();
    let policy = manager.policy();

    assert_eq!(policy.low_water_mark, 500);
    assert_eq!(policy.refill_batch_size, 2000);
    assert_eq!(policy.critical_low_threshold, 50);
}

#[test]
fn alert_levels_and_refill_plan_are_consistent() {
    let manager = AddressPoolManager::new(AddressPoolPolicy {
        low_water_mark: 500,
        refill_batch_size: 2000,
        critical_low_threshold: 50,
    });

    assert_eq!(manager.classify_alert(700), PoolAlertLevel::Healthy);
    assert_eq!(manager.classify_alert(300), PoolAlertLevel::Low);
    assert_eq!(manager.classify_alert(10), PoolAlertLevel::Critical);

    assert_eq!(manager.refill_plan(499), Some(2000));
    assert_eq!(manager.refill_plan(500), None);
}
