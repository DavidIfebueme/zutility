use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use tokio::sync::Mutex;

use super::{BlockchainInfo, ReceivedNote, ZcashClient};

#[derive(Debug, Clone)]
pub struct MockZcashClient {
    network: crate::config::ZcashNetwork,
    auto_confirm: bool,
    address_counter: Arc<AtomicU64>,
    confirmed_payments: Arc<Mutex<std::collections::HashMap<String, Decimal>>>,
    generated_addresses: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl MockZcashClient {
    pub fn new(network: crate::config::ZcashNetwork, auto_confirm: bool) -> Self {
        Self {
            network,
            auto_confirm,
            address_counter: Arc::new(AtomicU64::new(1)),
            confirmed_payments: Arc::new(Mutex::new(std::collections::HashMap::new())),
            generated_addresses: Arc::new(Mutex::new(std::collections::HashSet::new())),
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
        let address = self.transparent_address(self.next_address_index());
        self.generated_addresses
            .lock()
            .await
            .insert(address.clone());
        Ok(address)
    }

    async fn generate_shielded_address(&self) -> Result<String> {
        let address = self.shielded_address(self.next_address_index());
        self.generated_addresses
            .lock()
            .await
            .insert(address.clone());
        Ok(address)
    }

    async fn scan_payments(&self, _since_timestamp: u32) -> Result<Vec<ReceivedNote>> {
        if self.auto_confirm {
            let addresses = self.generated_addresses.lock().await;
            return Ok(addresses
                .iter()
                .map(|address| ReceivedNote {
                    txid: format!("mocktxid{address}"),
                    address: address.clone(),
                    amount: Decimal::new(1_000_000_000, 8),
                    confirmations: 10,
                    memo: None,
                })
                .collect());
        }

        let payments = self.confirmed_payments.lock().await;
        Ok(payments
            .iter()
            .map(|(address, amount)| ReceivedNote {
                txid: format!("mocktxid{address}"),
                address: address.clone(),
                amount: *amount,
                confirmations: 10,
                memo: None,
            })
            .collect())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}
