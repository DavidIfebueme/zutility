use zutility_be::{config, db, domain, http, integrations, jobs, observability, ws};

#[test]
fn module_tree_is_accessible() {
    let _ = std::mem::size_of::<config::AppConfig>();
    let _ = std::mem::size_of::<db::DbProvider>();
    let _ = std::mem::size_of::<domain::order::OrderStatus>();
    let _ = http::router();
    let _ = std::mem::size_of::<jobs::WorkerKind>();
    let _ = std::mem::size_of::<integrations::zcash::ZcashNetwork>();
    let _ = std::mem::size_of::<ws::WsEvent>();
    let _ = observability::init_tracing as fn();
}
