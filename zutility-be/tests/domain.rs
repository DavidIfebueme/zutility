use proptest::prelude::*;
use rust_decimal::Decimal;
use zutility_be::domain::order::{
    OrderStatus, OrderStatusTransition, PaymentAmountState, ThresholdError, ThresholdPolicy,
};

#[test]
fn builds_valid_transition_for_allowed_path() {
    let transition =
        OrderStatusTransition::new(OrderStatus::PaymentDetected, OrderStatus::PaymentConfirmed);
    assert!(transition.is_ok());
}

#[test]
fn rejects_invalid_transition_path() {
    let transition =
        OrderStatusTransition::new(OrderStatus::Completed, OrderStatus::AwaitingPayment);
    assert!(transition.is_err());
}

#[test]
fn threshold_policy_validation_rejects_invalid_basis_points() {
    let policy = ThresholdPolicy {
        underpay_tolerance_bps: 10_001,
        overpay_tolerance_bps: 50,
        late_payment_grace_minutes: 120,
    };

    let result = policy.validate();
    assert!(matches!(result, Err(ThresholdError::InvalidBasisPoints)));
}

#[test]
fn threshold_policy_classifies_underpaid_inrange_overpaid() {
    let policy = ThresholdPolicy::default()
        .validate()
        .expect("policy should be valid");
    let expected = Decimal::new(100_000_000, 8);

    let underpaid = policy.classify_received_amount(expected, Decimal::new(99_000_000, 8));
    let in_range = policy.classify_received_amount(expected, Decimal::new(99_500_000, 8));
    let overpaid = policy.classify_received_amount(expected, Decimal::new(101_000_000, 8));

    assert_eq!(underpaid, PaymentAmountState::Underpaid);
    assert_eq!(in_range, PaymentAmountState::InRange);
    assert_eq!(overpaid, PaymentAmountState::Overpaid);
}

proptest! {
    #[test]
    fn threshold_bounds_and_classification_are_consistent(
        expected_minor in 1_i64..10_000_000_i64,
        underpay_bps in 0_u16..=1000_u16,
        overpay_bps in 0_u16..=1000_u16,
    ) {
        let policy = ThresholdPolicy {
            underpay_tolerance_bps: underpay_bps,
            overpay_tolerance_bps: overpay_bps,
            late_payment_grace_minutes: 120,
        }
        .validate()
        .expect("valid policy");

        let expected = Decimal::new(expected_minor, 8);
        let min = policy.min_required_amount(expected);
        let max = policy.max_expected_amount(expected);

        prop_assert!(min <= expected);
        prop_assert!(expected <= max);

        let below_min = min - Decimal::new(1, 8);
        let above_max = max + Decimal::new(1, 8);

        prop_assert_eq!(
            policy.classify_received_amount(expected, min),
            PaymentAmountState::InRange
        );
        prop_assert_eq!(
            policy.classify_received_amount(expected, max),
            PaymentAmountState::InRange
        );
        prop_assert_eq!(
            policy.classify_received_amount(expected, below_min),
            PaymentAmountState::Underpaid
        );
        prop_assert_eq!(
            policy.classify_received_amount(expected, above_max),
            PaymentAmountState::Overpaid
        );
    }
}
