use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use tokio::sync::Mutex;

use super::{BlockchainInfo, ReceivedNote, TransparentPaymentObservation, ZcashClient};

#[derive(Debug, Clone)]
pub struct MockZcashClient {
    network: crate::config::ZcashNetwork,
    auto_confirm: bool,
    address_counter: Arc<AtomicU64>,
    confirmed_payments: Arc<Mutex<std::collections::HashMap<String, Decimal>>>,
}

impl MockZcashClient {
    pub fn new(network: crate::config::ZcashNetwork, auto_confirm: bool) -> Self {
        Self {
            network,
            auto_confirm,
            address_counter: Arc::new(AtomicU64::new(1)),
            confirmed_payments: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn next_address_index(&self) -> u64 {
        self.address_counter.fetch_add(1, Ordering::SeqCst)
    }

    fn transparent_address(&self, index: u64) -> String {
        match self.network {
            crate::config::ZcashNetwork::Mainnet => {
                format!("t1MockZcashTransparentAddress{:024x}", index)
            }
            crate::config::ZcashNetwork::Testnet => {
                format!("tmMockZcashTransparentAddress{:024x}", index)
            }
        }
    }

    fn shielded_address(&self, index: u64) -> String {
        match self.network {
            crate::config::ZcashNetwork::Mainnet => {
                format!("zsMockZcashShieldedAddress{:024x}", index)
            }
            crate::config::ZcashNetwork::Testnet => {
                format!("ztestsaplingMockAddress{:024x}", index)
            }
        }
    }

    pub async fn confirm_payment(&self, address: &str, amount: Decimal) {
        let mut payments = self.confirmed_payments.lock().await;
        payments.insert(address.to_owned(), amount);
    }
}

#[async_trait]
impl ZcashClient for MockZcashClient {
    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn save_wallet(&self) -> Result<()> {
        Ok(())
    }

    async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        let chain = match self.network {
            crate::config::ZcashNetwork::Mainnet => "main",
            crate::config::ZcashNetwork::Testnet => "test",
        };
        Ok(BlockchainInfo {
            chain: chain.to_owned(),
            blocks: 2_500_000,
            headers: 2_500_000,
            verification_progress: 1.0,
        })
    }

    async fn generate_transparent_address(&self) -> Result<String> {
        Ok(self.transparent_address(self.next_address_index()))
    }

    async fn generate_shielded_address(&self) -> Result<String> {
        Ok(self.shielded_address(self.next_address_index()))
    }

    async fn observe_transparent_payment(
        &self,
        address: &str,
        _current_block_height: u64,
        _since_timestamp: u32,
    ) -> Result<TransparentPaymentObservation> {
        let amount = if self.auto_confirm {
            Decimal::new(1_000_000_000, 8)
        } else {
            let payments = self.confirmed_payments.lock().await;
            payments.get(address).copied().unwrap_or(Decimal::ZERO)
        };

        Ok(TransparentPaymentObservation {
            total_received: amount,
            confirmations: if amount > Decimal::ZERO { 10 } else { 0 },
            utxo_count: if amount > Decimal::ZERO { 1 } else { 0 },
            has_mempool_tx: false,
        })
    }

    async fn list_received_by_address(
        &self,
        address: &str,
        _min_confirmations: u64,
        _since_timestamp: u32,
    ) -> Result<Vec<ReceivedNote>> {
        let amount = if self.auto_confirm {
            Decimal::new(1_000_000_000, 8)
        } else {
            let payments = self.confirmed_payments.lock().await;
            payments.get(address).copied().unwrap_or(Decimal::ZERO)
        };

        if amount <= Decimal::ZERO {
            return Ok(Vec::new());
        }

        Ok(vec![ReceivedNote {
            txid: format!("mocktxid{}", address),
            address: address.to_owned(),
            amount,
            confirmations: 10,
            memo: None,
        }])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
