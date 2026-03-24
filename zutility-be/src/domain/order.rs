#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
