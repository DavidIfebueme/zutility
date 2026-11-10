use anyhow::Result;
use async_trait::async_trait;

use super::{BlockchainInfo, ReceivedNote, ZcashClient, ZcashRpcClient};

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
        anyhow::bail!(
            "transparent address generation not supported via zcashd RPC adapter; pre-seed addresses instead"
        )
    }

    async fn generate_shielded_address(&self) -> Result<String> {
        self.inner.allocate_shielded_address(true).await
    }

    async fn scan_payments(&self, _since_timestamp: u32) -> Result<Vec<ReceivedNote>> {
        anyhow::bail!(
            "scan_payments is not supported by the legacy zcashd RPC backend; use ZCASH_BACKEND=zingolib"
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
