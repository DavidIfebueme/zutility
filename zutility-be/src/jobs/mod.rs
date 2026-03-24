#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerKind {
    ConfirmationWatcher,
    UtilityDispatcher,
    Sweeper,
    RateRefresher,
    OrderTimeoutReaper,
}
