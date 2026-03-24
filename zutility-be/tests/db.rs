use std::{fs, path::PathBuf};

use zutility_be::domain::order::OrderStatus;

fn repo_path(parts: &[&str]) -> PathBuf {
    parts.iter().fold(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        |mut acc, part| {
            acc.push(part);
            acc
        },
    )
}

#[test]
fn order_status_transitions_guard_rules_are_enforced() {
    assert!(OrderStatus::AwaitingPayment.can_transition_to(OrderStatus::PaymentDetected));
    assert!(OrderStatus::AwaitingPayment.can_transition_to(OrderStatus::Expired));
    assert!(OrderStatus::AwaitingPayment.can_transition_to(OrderStatus::Cancelled));
    assert!(OrderStatus::PaymentDetected.can_transition_to(OrderStatus::PaymentConfirmed));
    assert!(OrderStatus::PaymentDetected.can_transition_to(OrderStatus::FlaggedForReview));
    assert!(OrderStatus::PaymentConfirmed.can_transition_to(OrderStatus::UtilityDispatching));
    assert!(OrderStatus::UtilityDispatching.can_transition_to(OrderStatus::Completed));
    assert!(OrderStatus::UtilityDispatching.can_transition_to(OrderStatus::Failed));

    assert!(!OrderStatus::Completed.can_transition_to(OrderStatus::AwaitingPayment));
    assert!(!OrderStatus::Failed.can_transition_to(OrderStatus::Completed));
    assert!(!OrderStatus::Expired.can_transition_to(OrderStatus::PaymentDetected));
}

#[test]
fn migration_contains_required_tables_and_indexes() {
    let migration = fs::read_to_string(repo_path(&["migrations", "20260324170000_init_core.sql"]))
        .expect("read init migration");

    for required in [
        "CREATE TABLE IF NOT EXISTS orders",
        "CREATE TABLE IF NOT EXISTS rate_snapshots",
        "CREATE TABLE IF NOT EXISTS deposit_addresses",
        "CREATE TABLE IF NOT EXISTS audit_log",
        "CREATE INDEX IF NOT EXISTS idx_orders_status",
        "CHECK (amount_ngn > 0)",
        "CHECK (zec_amount > 0)",
        "orders_expiry_after_create",
    ] {
        assert!(migration.contains(required), "missing pattern: {required}");
    }
}

#[test]
fn idempotency_migration_contains_unique_request_id_and_attempts_table() {
    let migration = fs::read_to_string(repo_path(&[
        "migrations",
        "20260324170500_order_dispatch_controls.sql",
    ]))
    .expect("read dispatch migration");

    assert!(migration.contains("CREATE TABLE IF NOT EXISTS order_dispatch_attempts"));
    assert!(migration.contains("CREATE UNIQUE INDEX IF NOT EXISTS ux_orders_vtpass_request_id"));
}

#[test]
fn seed_files_exist_for_rates_utilities_and_deposit_addresses() {
    let seeds = [
        "seed_rate_snapshots.sql",
        "seed_utilities.sql",
        "seed_deposit_addresses.sql",
    ];

    for file in seeds {
        let path = repo_path(&["seeds", file]);
        assert!(path.exists(), "missing seed file: {}", path.display());
        let content = fs::read_to_string(path).expect("read seed file");
        assert!(!content.trim().is_empty());
    }
}
