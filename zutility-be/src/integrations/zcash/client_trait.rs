use async_trait::async_trait;
use anyhow::Result;

use super::{BlockchainInfo, ReceivedNote, TransparentPaymentObservation};

#[async_trait]
pub trait ZcashClient: Send + Sync {
    async fn sync(&self) -> Result<()>;

    async fn save_wallet(&self) -> Result<()>;

    async fn get_blockchain_info(&self) -> Result<BlockchainInfo>;

    async fn generate_transparent_address(&self) -> Result<String>;

    async fn generate_shielded_address(&self) -> Result<String>;

    async fn observe_transparent_payment(
        &self,
        address: &str,
        current_block_height: u64,
        since_timestamp: u32,
    ) -> Result<TransparentPaymentObservation>;

    async fn list_received_by_address(
        &self,
        address: &str,
        min_confirmations: u64,
        since_timestamp: u32,
    ) -> Result<Vec<ReceivedNote>>;
}
