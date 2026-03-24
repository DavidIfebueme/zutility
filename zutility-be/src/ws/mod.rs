#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsEvent {
    PaymentDetected,
    Confirmation,
    PaymentConfirmed,
    Dispatching,
    Completed,
    Expired,
    Failed,
}
