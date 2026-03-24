pub mod address_pool;
pub mod rate_refresher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerKind {
    ConfirmationWatcher,
    UtilityDispatcher,
    Sweeper,
    RateRefresher,
    OrderTimeoutReaper,
}
