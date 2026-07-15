use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::http::handlers::HttpState;

#[derive(Debug, Serialize)]
pub struct WalletBalanceResponse {
    pub transparent_balance: String,
    pub shielded_balance: String,
    pub total_balance: String,
    pub chain_tip: u64,
    pub address_pool: AddressPoolInfo,
}

#[derive(Debug, Serialize)]
pub struct AddressPoolInfo {
    pub shielded_unused: i64,
    pub transparent_unused: i64,
}

pub async fn wallet_balance(
    State(state): State<HttpState>,
) -> Result<Json<WalletBalanceResponse>, (StatusCode, String)> {
    let client = state
        .zcash_client
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "zcash client not configured".into()))?;

    let info = client
        .get_blockchain_info()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("blockchain info failed: {e}")))?;

    let balance_info = {
        let zingo = client
            .as_any()
            .downcast_ref::<crate::integrations::zcash::ZingoClient>()
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "not a zingo client".into()))?;

        zingo.get_wallet_balance()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("balance query failed: {e}")))?
    };

    let network = state.zcash_network.as_str();
    let shielded_unused = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deposit_addresses WHERE address_type = 'shielded' AND order_id IS NULL AND network = $1",
    )
    .bind(network)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    let transparent_unused = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deposit_addresses WHERE address_type = 'transparent' AND order_id IS NULL AND network = $1",
    )
    .bind(network)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    Ok(Json(WalletBalanceResponse {
        transparent_balance: balance_info.transparent,
        shielded_balance: balance_info.shielded,
        total_balance: balance_info.total,
        chain_tip: info.blocks,
        address_pool: AddressPoolInfo {
            shielded_unused,
            transparent_unused,
        },
    }))
}
