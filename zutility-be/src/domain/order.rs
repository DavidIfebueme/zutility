use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    AwaitingPayment,
    PaymentDetected,
    PaymentConfirmed,
    UtilityDispatching,
    Completed,
    Expired,
    Failed,
    FlaggedForReview,
    Cancelled,
}

impl OrderStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::AwaitingPayment => "awaiting_payment",
            Self::PaymentDetected => "payment_detected",
            Self::PaymentConfirmed => "payment_confirmed",
            Self::UtilityDispatching => "utility_dispatching",
            Self::Completed => "completed",
            Self::Expired => "expired",
            Self::Failed => "failed",
            Self::FlaggedForReview => "flagged_for_review",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::AwaitingPayment, Self::PaymentDetected)
                | (Self::AwaitingPayment, Self::Expired)
                | (Self::AwaitingPayment, Self::Cancelled)
                | (Self::PaymentDetected, Self::PaymentConfirmed)
                | (Self::PaymentDetected, Self::Expired)
                | (Self::PaymentDetected, Self::FlaggedForReview)
                | (Self::PaymentConfirmed, Self::UtilityDispatching)
                | (Self::UtilityDispatching, Self::Completed)
                | (Self::UtilityDispatching, Self::Failed)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderStatusTransition {
    pub from: OrderStatus,
    pub to: OrderStatus,
}

impl OrderStatusTransition {
    pub fn new(from: OrderStatus, to: OrderStatus) -> Result<Self, TransitionError> {
        if !from.can_transition_to(to) {
            return Err(TransitionError::InvalidTransition { from, to });
        }
        Ok(Self { from, to })
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    #[error("invalid status transition {from:?} -> {to:?}")]
    InvalidTransition { from: OrderStatus, to: OrderStatus },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThresholdPolicy {
    pub underpay_tolerance_bps: u16,
    pub overpay_tolerance_bps: u16,
    pub late_payment_grace_minutes: u16,
}

impl Default for ThresholdPolicy {
    fn default() -> Self {
        Self {
            underpay_tolerance_bps: 50,
            overpay_tolerance_bps: 50,
            late_payment_grace_minutes: 120,
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdError {
    #[error("threshold basis points cannot exceed 10000")]
    InvalidBasisPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentAmountState {
    Underpaid,
    InRange,
    Overpaid,
}

impl ThresholdPolicy {
    pub fn validate(self) -> Result<Self, ThresholdError> {
        if self.underpay_tolerance_bps > 10_000 || self.overpay_tolerance_bps > 10_000 {
            return Err(ThresholdError::InvalidBasisPoints);
        }
        Ok(self)
    }

    pub fn min_required_amount(self, expected: Decimal) -> Decimal {
        let bps = Decimal::from(10_000u32.saturating_sub(self.underpay_tolerance_bps as u32));
        (expected * bps) / Decimal::from(10_000u32)
    }

    pub fn max_expected_amount(self, expected: Decimal) -> Decimal {
        let bps = Decimal::from(10_000u32.saturating_add(self.overpay_tolerance_bps as u32));
        (expected * bps) / Decimal::from(10_000u32)
    }

    pub fn classify_received_amount(
        self,
        expected: Decimal,
        received: Decimal,
    ) -> PaymentAmountState {
        if received < self.min_required_amount(expected) {
            return PaymentAmountState::Underpaid;
        }
        if received > self.max_expected_amount(expected) {
            return PaymentAmountState::Overpaid;
        }
        PaymentAmountState::InRange
    }
}
