use anyhow::Result;
use async_trait::async_trait;

use super::{BlockchainInfo, ReceivedNote};

#[async_trait]
pub trait ZcashClient: Send + Sync {
    async fn sync(&self) -> Result<()>;

    async fn save_wallet(&self) -> Result<()>;

    async fn get_blockchain_info(&self) -> Result<BlockchainInfo>;

    async fn generate_transparent_address(&self) -> Result<String>;

    async fn generate_shielded_address(&self) -> Result<String>;

    async fn scan_payments(&self, since_timestamp: u32) -> Result<Vec<ReceivedNote>>;

    fn as_any(&self) -> &dyn std::any::Any;

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
