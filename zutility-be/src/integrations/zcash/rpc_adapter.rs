use async_trait::async_trait;
use anyhow::Result;

use super::{
    ZcashClient, ZcashRpcClient, BlockchainInfo, ReceivedNote, TransparentPaymentObservation,
};

pub struct ZcashRpcAdapter {
    inner: ZcashRpcClient,
}

impl ZcashRpcAdapter {
    pub fn new(inner: ZcashRpcClient) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl ZcashClient for ZcashRpcAdapter {
    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn save_wallet(&self) -> Result<()> {
        Ok(())
    }

    async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        self.inner.get_blockchain_info().await
    }

    async fn generate_transparent_address(&self) -> Result<String> {
        anyhow::bail!("transparent address generation not supported via zcashd RPC adapter; pre-seed addresses instead")
    }

    async fn generate_shielded_address(&self) -> Result<String> {
        self.inner.allocate_shielded_address(true).await
    }

    async fn observe_transparent_payment(
        &self,
        address: &str,
        current_block_height: u64,
        _since_timestamp: u32,
    ) -> Result<TransparentPaymentObservation> {
        self.inner
            .observe_transparent_payment(address, current_block_height)
            .await
    }

    async fn list_received_by_address(
        &self,
        address: &str,
        min_confirmations: u64,
        _since_timestamp: u32,
    ) -> Result<Vec<ReceivedNote>> {
        self.inner.list_received_by_address(address, min_confirmations).await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
