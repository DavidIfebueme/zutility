use std::sync::Arc;

use async_trait::async_trait;
use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use zingolib::config::{ChainType, ClientConfig, WalletConfig};
use zingolib::lightclient::LightClient;
use zingolib::wallet::keys::unified::ReceiverSelection;
use zingolib::wallet::summary::data::TransactionKind;

use super::{
    ZcashClient, BlockchainInfo, ReceivedNote, TransparentPaymentObservation,
    ZATOSHI_PER_ZEC,
};

pub struct ZingoClient {
    client: Arc<RwLock<LightClient>>,
    chain_type: ChainType,
    chain_tip: Arc<RwLock<u64>>,
}

impl ZingoClient {
    pub async fn new(indexer_uri: &str, wallet_dir: &str, chain_type: ChainType) -> Result<Self> {
        let uri = indexer_uri
            .parse::<axum::http::Uri>()
            .context("invalid zingolib indexer URI")?;

        let wallet_path = std::path::PathBuf::from(wallet_dir).join("zingo-wallet.dat");
        let wallet_exists = wallet_path.exists();

        let wallet_config = if wallet_exists {
            tracing::info!("loading existing zingolib wallet from {}", wallet_path.display());
            WalletConfig::Read
        } else {
            tracing::info!("creating new zingolib wallet at {}", wallet_path.display());
            WalletConfig::NewSeed {
                no_of_accounts: std::num::NonZeroU32::new(1).context("invalid account count")?,
                chain_height: 0,
                wallet_settings: Default::default(),
            }
        };

        let config = ClientConfig::builder()
            .set_indexer_uri(uri)
            .set_chain_type(chain_type)
            .set_wallet_dir(std::path::PathBuf::from(wallet_dir))
            .set_wallet_config(wallet_config)
            .build();

        let overwrite = !wallet_exists;
        let mut client = LightClient::new(config, overwrite)
            .map_err(|e| anyhow::anyhow!("zingolib LightClient init failed: {e}"))?;

        if !wallet_exists {
            client.save_task().await;
        }

        client.sync().await
            .map_err(|e| anyhow::anyhow!("zingolib initial sync failed: {e}"))?;

        let zingo = Self {
            client: Arc::new(RwLock::new(client)),
            chain_type,
            chain_tip: Arc::new(RwLock::new(0)),
        };

        zingo.update_chain_tip().await?;

        Ok(zingo)
    }

    pub async fn sync(&self) -> Result<()> {
        let mut client = self.client.write().await;
        client.sync().await
            .map_err(|e| anyhow::anyhow!("zingolib sync failed: {e}"))?;
        drop(client);
        self.update_chain_tip().await?;
        Ok(())
    }

    pub async fn save_wallet(&self) -> Result<()> {
        let mut client = self.client.write().await;
        client.save_task().await;
        Ok(())
    }

    async fn update_chain_tip(&self) -> Result<()> {
        let client = self.client.read().await;
        let info_str = client.do_info().await;
        drop(client);

        if let Some(height) = parse_chain_tip_from_info(&info_str) {
            let mut tip = self.chain_tip.write().await;
            *tip = height;
        }

        Ok(())
    }

    async fn current_chain_tip(&self) -> u64 {
        *self.chain_tip.read().await
    }
}

fn parse_chain_tip_from_info(info: &str) -> Option<u64> {
    for line in info.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("latest_block_height:") || trimmed.starts_with("\"latest_block_height\"") {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                return parts[1].trim().trim_matches('"').trim().parse().ok();
            }
        }
    }
    None
}

fn zatoshis_to_zec(zat: u64) -> Decimal {
    Decimal::from(zat) / Decimal::from(ZATOSHI_PER_ZEC)
}

fn confirmation_count_from_status(status: &zingo_status::confirmation_status::ConfirmationStatus, chain_tip: u64) -> u64 {
    match status {
        zingo_status::confirmation_status::ConfirmationStatus::Confirmed(height) => {
            let confirmed_at: u32 = (*height).into();
            if chain_tip >= confirmed_at as u64 {
                chain_tip - confirmed_at as u64 + 1
            } else {
                1
            }
        }
        zingo_status::confirmation_status::ConfirmationStatus::Mempool(_) => 0,
        zingo_status::confirmation_status::ConfirmationStatus::Transmitted(_) => 0,
        zingo_status::confirmation_status::ConfirmationStatus::Calculated(_) => 0,
        zingo_status::confirmation_status::ConfirmationStatus::Failed(_) => 0,
    }
}

#[async_trait]
impl ZcashClient for ZingoClient {
    async fn sync(&self) -> Result<()> {
        self.sync().await
    }

    async fn save_wallet(&self) -> Result<()> {
        self.save_wallet().await
    }

    async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        let client = self.client.read().await;
        let _info_str = client.do_info().await;
        drop(client);

        let chain_tip = self.current_chain_tip().await;

        let chain = if matches!(self.chain_type, ChainType::Testnet) {
            "test"
        } else {
            "main"
        };

        Ok(BlockchainInfo {
            chain: chain.to_owned(),
            blocks: chain_tip,
            headers: chain_tip,
            verification_progress: if chain_tip > 0 { 1.0 } else { 0.0 },
        })
    }

    async fn generate_transparent_address(&self) -> Result<String> {
        let mut client = self.client.write().await;
        let account_id = zip32::AccountId::try_from(0u32)
            .map_err(|e| anyhow::anyhow!("invalid account id: {e:?}"))?;

        let (_, address) = client
            .generate_transparent_address(account_id, true)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib transparent address generation failed: {e}"))?;

        Ok(pepper_sync::keys::transparent::encode_address(&self.chain_type, address))
    }

    async fn generate_shielded_address(&self) -> Result<String> {
        let mut client = self.client.write().await;
        let account_id = zip32::AccountId::try_from(0u32)
            .map_err(|e| anyhow::anyhow!("invalid account id: {e:?}"))?;

        let receivers = ReceiverSelection::orchard_only();
        let (_, unified_address) = client
            .generate_unified_address(receivers, account_id)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib unified address generation failed: {e}"))?;

        Ok(unified_address.encode(&self.chain_type))
    }

    async fn observe_transparent_payment(
        &self,
        _address: &str,
        current_block_height: u64,
    ) -> Result<TransparentPaymentObservation> {
        let client = self.client.read().await;
        let account_id = zip32::AccountId::try_from(0u32)
            .map_err(|e| anyhow::anyhow!("invalid account id: {e:?}"))?;

        let balance = client
            .account_balance(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib account_balance failed: {e}"))?;

        let confirmed_zat = balance
            .confirmed_transparent_balance
            .map(|z| u64::from(z))
            .unwrap_or(0);
        let unconfirmed_zat = balance
            .unconfirmed_transparent_balance
            .map(|z| u64::from(z))
            .unwrap_or(0);

        let has_mempool_tx = unconfirmed_zat > confirmed_zat;

        let total_received = zatoshis_to_zec(confirmed_zat);

        let confirmations = if confirmed_zat > 0 && current_block_height > 0 {
            let summaries = client
                .transaction_summaries(false)
                .await
                .map_err(|e| anyhow::anyhow!("zingolib transaction_summaries failed: {e}"))?;

            let min_confirmed_height = summaries
                .iter()
                .filter(|tx| matches!(tx.kind, TransactionKind::Received))
                .filter_map(|tx| {
                    if let zingo_status::confirmation_status::ConfirmationStatus::Confirmed(h) = tx.status {
                        Some(u64::from(u32::from(h)))
                    } else {
                        None
                    }
                })
                .min()
                .unwrap_or(0);

            if min_confirmed_height > 0 {
                u16::try_from(current_block_height.saturating_sub(min_confirmed_height) + 1)
                    .unwrap_or(u16::MAX)
            } else {
                0
            }
        } else {
            0
        };

        Ok(TransparentPaymentObservation {
            total_received,
            confirmations,
            utxo_count: if confirmed_zat > 0 { 1 } else { 0 },
            has_mempool_tx,
        })
    }

    async fn list_received_by_address(
        &self,
        _address: &str,
        min_confirmations: u64,
    ) -> Result<Vec<ReceivedNote>> {
        let client = self.client.read().await;

        let summaries = client
            .transaction_summaries(false)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib transaction_summaries failed: {e}"))?;

        let chain_tip = self.current_chain_tip().await;

        let notes: Vec<ReceivedNote> = summaries
            .iter()
            .filter(|tx| matches!(tx.kind, TransactionKind::Received))
            .filter(|tx| {
                let confs = confirmation_count_from_status(&tx.status, chain_tip);
                confs >= min_confirmations
            })
            .map(|tx| {
                let confs = confirmation_count_from_status(&tx.status, chain_tip);
                ReceivedNote {
                    txid: tx.txid.to_string(),
                    address: String::new(),
                    amount: zatoshis_to_zec(tx.value),
                    confirmations: confs,
                    memo: None,
                }
            })
            .collect();

        Ok(notes)
    }
}
