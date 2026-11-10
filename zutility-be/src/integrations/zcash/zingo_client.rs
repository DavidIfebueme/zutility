use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use pepper_sync::wallet::{NoteInterface, OutputInterface};
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use zingolib::config::{ChainType, ClientConfig, WalletConfig};
use zingolib::lightclient::LightClient;
use zingolib::wallet::keys::unified::ReceiverSelection;
use zingolib::wallet::summary::data::TransactionKind;

use super::{BlockchainInfo, ReceivedNote, WalletBalanceInfo, ZATOSHI_PER_ZEC, ZcashClient};

#[derive(Clone)]
struct RetryPolicy {
    max_retries: u8,
    delay: Duration,
}

impl RetryPolicy {
    fn new(max_retries: u8, delay_ms: u64) -> Self {
        Self {
            max_retries,
            delay: Duration::from_millis(delay_ms),
        }
    }
}

async fn retry<F, Fut>(policy: &RetryPolicy, label: &str, mut f: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut last_err = None;
    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let is_last = attempt == policy.max_retries;
                if is_last {
                    last_err = Some(e);
                } else {
                    tracing::warn!(
                        attempt,
                        max_retries = policy.max_retries,
                        delay_ms = policy.delay.as_millis(),
                        error = %e,
                        "{label} failed, retrying"
                    );
                    tokio::time::sleep(policy.delay).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("{label} failed without explicit error")))
}

pub struct ZingoClient {
    client: Arc<RwLock<LightClient>>,
    chain_type: ChainType,
    chain_tip: Arc<RwLock<u64>>,
    retry_policy: RetryPolicy,
}

impl ZingoClient {
    pub async fn new(
        indexer_uri: &str,
        wallet_dir: &str,
        chain_type: ChainType,
        wallet_birthday: u32,
        sync_retries: u8,
        sync_retry_delay_ms: u64,
    ) -> Result<Self> {
        let uri = indexer_uri
            .parse::<axum::http::Uri>()
            .context("invalid zingolib indexer URI")?;

        let network_name = match chain_type {
            ChainType::Testnet => "testnet",
            ChainType::Mainnet => "mainnet",
            _ => "unknown",
        };
        let wallet_dir = std::path::PathBuf::from(wallet_dir).join(network_name);
        std::fs::create_dir_all(&wallet_dir).with_context(|| {
            format!("failed to create wallet directory {}", wallet_dir.display())
        })?;

        let wallet_path = wallet_dir.join("zingo-wallet.dat");
        let wallet_exists = wallet_path.exists();

        let wallet_config = if wallet_exists {
            tracing::info!(
                "loading existing zingolib wallet from {}",
                wallet_path.display()
            );
            WalletConfig::Read
        } else {
            tracing::info!("creating new zingolib wallet at {}", wallet_path.display());
            WalletConfig::NewSeed {
                no_of_accounts: std::num::NonZeroU32::new(1).context("invalid account count")?,
                chain_height: wallet_birthday,
                wallet_settings: Default::default(),
            }
        };

        let config = ClientConfig::builder()
            .set_indexer_uri(uri)
            .set_chain_type(chain_type)
            .set_wallet_dir(wallet_dir.clone())
            .set_wallet_config(wallet_config)
            .build()
            .map_err(|e| anyhow::anyhow!("zingolib client config failed: {e}"))?;

        let overwrite = !wallet_exists;
        let mut client = LightClient::new(config, overwrite)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib LightClient init failed: {e}"))?;

        if !wallet_exists {
            client.save_task().await;
            if let Some(seed) = client.mnemonic_phrase() {
                let seed_path = wallet_dir.join("seed.txt");
                if let Err(error) = std::fs::write(&seed_path, &seed) {
                    tracing::error!(path = %seed_path.display(), error = %error, "failed to write wallet seed backup");
                } else {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &seed_path,
                            std::fs::Permissions::from_mode(0o600),
                        );
                    }
                    tracing::warn!(path = %seed_path.display(), "new zingolib wallet created — seed phrase written to file; back it up securely and remove from server if possible");
                }
            } else {
                tracing::warn!(
                    "new zingolib wallet created but no mnemonic phrase was available — ensure wallet is recoverable"
                );
            }
        }

        let retry_policy = RetryPolicy::new(sync_retries, sync_retry_delay_ms);
        {
            let mut last_err = None;
            for attempt in 0..=retry_policy.max_retries {
                match client.sync_and_await().await {
                    Ok(_) => break,
                    Err(e) => {
                        let is_last = attempt == retry_policy.max_retries;
                        if is_last {
                            last_err = Some(e);
                        } else {
                            tracing::warn!(
                                attempt,
                                max_retries = retry_policy.max_retries,
                                delay_ms = retry_policy.delay.as_millis(),
                                error = %e,
                                "initial sync failed, retrying"
                            );
                            tokio::time::sleep(retry_policy.delay).await;
                        }
                    }
                }
            }
            if let Some(e) = last_err {
                return Err(anyhow::anyhow!(
                    "zingolib initial sync failed after {} retries: {e}",
                    retry_policy.max_retries
                ));
            }
        }

        let zingo = Self {
            client: Arc::new(RwLock::new(client)),
            chain_type,
            chain_tip: Arc::new(RwLock::new(0)),
            retry_policy,
        };

        zingo.update_chain_tip().await?;

        Ok(zingo)
    }

    pub async fn sync(&self) -> Result<()> {
        let retry_policy = self.retry_policy.clone();
        let client = self.client.clone();
        let chain_tip = self.chain_tip.clone();

        retry(&retry_policy, "sync", || {
            let client = client.clone();
            let chain_tip = chain_tip.clone();
            async move {
                let mut cl = client.write().await;
                match cl.sync_and_await().await {
                    Ok(_) => {}
                    Err(e) => {
                        let msg = format!("{e}");
                        if msg.contains("sync is already running") {
                            tracing::debug!("zingolib sync skipped — already in progress");
                        } else {
                            return Err(anyhow::anyhow!("zingolib sync failed: {e}"));
                        }
                    }
                }

                let info = cl
                    .info()
                    .await
                    .map_err(|e| anyhow::anyhow!("zingolib info failed: {e}"))?;
                drop(cl);

                let mut tip = chain_tip.write().await;
                *tip = info.latest_block_height;

                Ok(())
            }
        })
        .await
    }

    pub async fn save_wallet(&self) -> Result<()> {
        let mut client = self.client.write().await;
        client.save_task().await;
        Ok(())
    }

    async fn update_chain_tip(&self) -> Result<()> {
        let retry_policy = self.retry_policy.clone();
        let client = self.client.clone();
        let chain_tip = self.chain_tip.clone();

        retry(&retry_policy, "update_chain_tip", || {
            let client = client.clone();
            let chain_tip = chain_tip.clone();
            async move {
                let mut cl = client.write().await;
                let info = cl
                    .info()
                    .await
                    .map_err(|e| anyhow::anyhow!("zingolib info failed: {e}"))?;
                drop(cl);

                let mut tip = chain_tip.write().await;
                *tip = info.latest_block_height;

                Ok(())
            }
        })
        .await
    }

    pub async fn get_wallet_balance(&self) -> Result<WalletBalanceInfo> {
        let client = self.client.read().await;
        let balance = client
            .account_balance(zip32::AccountId::ZERO)
            .await
            .map_err(|e| anyhow::anyhow!("account_balance failed: {e}"))?;
        drop(client);

        let transparent_zats = balance
            .total_transparent_balance
            .as_ref()
            .map(|z| z.into_u64())
            .unwrap_or(0);
        let shielded_zats = balance
            .total_ironwood_balance
            .as_ref()
            .map(|z| z.into_u64())
            .unwrap_or(0)
            + balance
                .total_orchard_balance
                .as_ref()
                .map(|z| z.into_u64())
                .unwrap_or(0)
            + balance
                .total_sapling_balance
                .as_ref()
                .map(|z| z.into_u64())
                .unwrap_or(0);
        let total_zats = transparent_zats + shielded_zats;

        Ok(WalletBalanceInfo {
            transparent: zatoshis_to_zec(transparent_zats).round_dp(8).to_string(),
            shielded: zatoshis_to_zec(shielded_zats).round_dp(8).to_string(),
            total: zatoshis_to_zec(total_zats).round_dp(8).to_string(),
        })
    }

    async fn current_chain_tip(&self) -> u64 {
        *self.chain_tip.read().await
    }
}

fn zatoshis_to_zec(zat: u64) -> Decimal {
    Decimal::from(zat) / Decimal::from(ZATOSHI_PER_ZEC)
}

fn confirmation_count_from_status(
    status: &zingo_status::confirmation_status::ConfirmationStatus,
    chain_tip: u64,
) -> u64 {
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
        let mut client = self.client.write().await;
        let info = client
            .info()
            .await
            .map_err(|e| anyhow::anyhow!("zingolib info failed: {e}"))?;
        drop(client);

        let chain_tip = info.latest_block_height;
        *self.chain_tip.write().await = chain_tip;

        Ok(BlockchainInfo {
            chain: info.chain_name,
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

        Ok(pepper_sync::keys::transparent::encode_address(
            &self.chain_type,
            address,
        ))
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

    async fn scan_payments(&self, since_timestamp: u32) -> Result<Vec<ReceivedNote>> {
        let client = self.client.read().await;
        let wallet = client.wallet().read().await;
        let chain_tip = self.current_chain_tip().await;
        let received_transactions: std::collections::HashSet<_> = wallet
            .transaction_summaries(false)
            .await
            .map_err(|e| anyhow::anyhow!("zingolib transaction summaries failed: {e}"))?
            .iter()
            .filter(|summary| matches!(summary.kind, TransactionKind::Received))
            .map(|summary| summary.txid)
            .collect();

        let mut notes = Vec::new();
        for tx in wallet.wallet_transactions.values() {
            if tx.datetime() < since_timestamp {
                continue;
            }
            if !received_transactions.contains(&tx.txid()) {
                continue;
            }

            let confirmations = confirmation_count_from_status(&tx.status(), chain_tip);
            for note in tx.ironwood_notes() {
                let recipient = note.note().recipient();
                if let Some(address) = wallet
                    .unified_addresses()
                    .values()
                    .find(|address| address.orchard() == Some(&recipient))
                    .map(|address| address.encode(&self.chain_type))
                {
                    notes.push(ReceivedNote {
                        txid: tx.txid().to_string(),
                        address,
                        amount: zatoshis_to_zec(note.value()),
                        confirmations,
                        memo: None,
                    });
                }
            }
            for note in tx.orchard_notes() {
                let recipient = note.note().recipient();
                if let Some(address) = wallet
                    .unified_addresses()
                    .values()
                    .find(|address| address.orchard() == Some(&recipient))
                    .map(|address| address.encode(&self.chain_type))
                {
                    notes.push(ReceivedNote {
                        txid: tx.txid().to_string(),
                        address,
                        amount: zatoshis_to_zec(note.value()),
                        confirmations,
                        memo: None,
                    });
                }
            }
            for note in tx.sapling_notes() {
                let recipient = note.note().recipient();
                if let Some(address) = wallet
                    .unified_addresses()
                    .values()
                    .find(|address| address.sapling() == Some(&recipient))
                    .map(|address| address.encode(&self.chain_type))
                {
                    notes.push(ReceivedNote {
                        txid: tx.txid().to_string(),
                        address,
                        amount: zatoshis_to_zec(note.value()),
                        confirmations,
                        memo: None,
                    });
                }
            }
            for note in tx.transparent_coins() {
                notes.push(ReceivedNote {
                    txid: tx.txid().to_string(),
                    address: note.address().to_owned(),
                    amount: zatoshis_to_zec(note.value()),
                    confirmations,
                    memo: None,
                });
            }
        }

        Ok(notes)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn health_check(&self) -> Result<()> {
        let mut client = self.client.write().await;
        let info = client
            .info()
            .await
            .map_err(|e| anyhow::anyhow!("zingolib info failed: {e}"))?;
        drop(client);

        if info.latest_block_height > 0 {
            tracing::debug!(
                chain_tip = info.latest_block_height,
                "zingolib health check passed"
            );
            Ok(())
        } else {
            anyhow::bail!("zingolib health check failed, indexer returned no block height")
        }
    }
}
