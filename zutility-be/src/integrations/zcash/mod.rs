use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use rust_decimal::Decimal;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::config::{AppConfig, AppEnv, ZcashRpcMode as ConfigRpcMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZcashNetwork {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZcashRpcMode {
    Unix,
    Tcp,
}

#[derive(Debug, Clone)]
pub struct ZcashRpcConfig {
    pub mode: ZcashRpcMode,
    pub socket_path: String,
    pub rpc_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
    pub network: ZcashNetwork,
}

#[derive(Debug, Clone)]
pub struct RpcRetryPolicy {
    pub max_retries: u8,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ZcashRpcClient {
    client: Client,
    config: ZcashRpcConfig,
    retry_policy: RpcRetryPolicy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockchainInfo {
    pub chain: String,
    pub blocks: u64,
    pub headers: u64,
    #[serde(rename = "verificationprogress")]
    pub verification_progress: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedNote {
    pub txid: String,
    pub address: String,
    pub amount: Decimal,
    pub confirmations: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentMatchStatus {
    Underpaid,
    Exact,
    Overpaid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaymentMatch {
    pub status: PaymentMatchStatus,
    pub total_received: Decimal,
    pub expected: Decimal,
    pub note_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: String,
    method: String,
    params: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcResponse {
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

impl ZcashRpcConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        let mode = match config.zcash_rpc_mode {
            ConfigRpcMode::Unix => ZcashRpcMode::Unix,
            ConfigRpcMode::Tcp => ZcashRpcMode::Tcp,
        };
        let network = match config.zcash_network {
            crate::config::ZcashNetwork::Mainnet => ZcashNetwork::Mainnet,
            crate::config::ZcashNetwork::Testnet => ZcashNetwork::Testnet,
        };

        Self {
            mode,
            socket_path: config.zcash_rpc_socket_path.clone(),
            rpc_url: config.zcash_rpc_url.clone(),
            rpc_user: config.zcash_rpc_user.expose_secret().to_owned(),
            rpc_password: config.zcash_rpc_password.expose_secret().to_owned(),
            network,
        }
    }
}

impl ZcashRpcClient {
    pub fn new(config: ZcashRpcConfig, retry_policy: RpcRetryPolicy) -> Result<Self> {
        let mut builder = Client::builder().timeout(retry_policy.timeout);

        if matches!(config.mode, ZcashRpcMode::Unix) {
            #[cfg(target_family = "unix")]
            {
                builder = builder.unix_socket(config.socket_path.clone());
            }
        }

        let client = builder
            .build()
            .context("failed to build zcash rpc client")?;
        Ok(Self {
            client,
            config,
            retry_policy,
        })
    }

    pub fn from_app_config(config: &AppConfig) -> Result<Self> {
        let retry_policy = RpcRetryPolicy {
            max_retries: 2,
            timeout: Duration::from_millis(config.rate_source_timeout_ms),
        };
        Self::new(ZcashRpcConfig::from_app_config(config), retry_policy)
    }

    pub fn mode(&self) -> ZcashRpcMode {
        self.config.mode
    }

    pub fn socket_path(&self) -> &str {
        &self.config.socket_path
    }

    pub async fn get_blockchain_info(&self) -> Result<BlockchainInfo> {
        let value = self.call("getblockchaininfo", json!([])).await?;
        serde_json::from_value::<BlockchainInfo>(value)
            .context("invalid response payload for getblockchaininfo")
    }

    pub async fn health_check_testnet(&self, minimum_sync_progress: f64) -> Result<BlockchainInfo> {
        let info = self.get_blockchain_info().await?;
        if info.chain != "test" {
            anyhow::bail!("zcash rpc chain is not testnet");
        }
        if info.verification_progress < minimum_sync_progress {
            anyhow::bail!("zcash rpc sync progress below threshold");
        }
        Ok(info)
    }

    pub async fn z_getnewaccount(&self) -> Result<u32> {
        let value = self.call("z_getnewaccount", json!([])).await?;
        if let Some(account) = value.as_u64() {
            return u32::try_from(account).context("account id out of range");
        }
        if let Some(text) = value.as_str() {
            return text
                .parse::<u32>()
                .context("failed to parse z_getnewaccount response as u32");
        }
        anyhow::bail!("unsupported z_getnewaccount response type")
    }

    pub async fn z_getaddressforaccount(&self, account: u32) -> Result<String> {
        let value = self
            .call("z_getaddressforaccount", json!([account]))
            .await?;
        value
            .as_str()
            .map(ToOwned::to_owned)
            .context("invalid z_getaddressforaccount response")
    }

    pub async fn z_getnewaddress_deprecated(&self) -> Result<String> {
        let value = self.call("z_getnewaddress", json!(["sapling"])).await?;
        value
            .as_str()
            .map(ToOwned::to_owned)
            .context("invalid z_getnewaddress response")
    }

    pub async fn allocate_shielded_address(
        &self,
        allow_deprecated_fallback: bool,
    ) -> Result<String> {
        match self.z_getnewaccount().await {
            Ok(account) => self.z_getaddressforaccount(account).await,
            Err(error) if allow_deprecated_fallback => {
                tracing::warn!(error = %error, "falling back to z_getnewaddress");
                self.z_getnewaddress_deprecated().await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn import_viewing_key(&self, viewing_key: &str) -> Result<()> {
        let _ = self
            .call(
                "z_importviewingkey",
                json!([viewing_key, "whenkeyisnew", 0]),
            )
            .await?;
        Ok(())
    }

    pub async fn generate_shielded_pool_addresses(
        &self,
        count: usize,
        allow_deprecated_fallback: bool,
    ) -> Result<Vec<String>> {
        let mut addresses = Vec::with_capacity(count);
        for _ in 0..count {
            let address = self
                .allocate_shielded_address(allow_deprecated_fallback)
                .await?;
            if !address.starts_with("ztestsapling") && self.config.network == ZcashNetwork::Testnet
            {
                anyhow::bail!("generated shielded address is not testnet");
            }
            addresses.push(address);
        }
        Ok(addresses)
    }

    pub async fn list_received_by_address(
        &self,
        address: &str,
        min_confirmations: u64,
    ) -> Result<Vec<ReceivedNote>> {
        let value = self
            .call(
                "z_listreceivedbyaddress",
                json!([address, min_confirmations]),
            )
            .await?;

        let notes = value
            .as_array()
            .context("z_listreceivedbyaddress result must be an array")?
            .iter()
            .map(parse_received_note)
            .collect::<Result<Vec<_>>>()?;

        Ok(notes
            .into_iter()
            .filter(|note| note.confirmations >= min_confirmations)
            .collect())
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let mut last_error: Option<anyhow::Error> = None;
        for attempt in 0..=self.retry_policy.max_retries {
            match self.call_once(method, params.clone()).await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < self.retry_policy.max_retries {
                        tokio::time::sleep(Duration::from_millis(150 * (attempt as u64 + 1))).await;
                    }
                }
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => anyhow::bail!("rpc call failed without explicit error"),
        }
    }

    async fn call_once(&self, method: &str, params: Value) -> Result<Value> {
        let request = RpcRequest {
            jsonrpc: "1.0",
            id: Uuid::new_v4().to_string(),
            method: method.to_owned(),
            params,
        };

        let endpoint = match self.config.mode {
            ZcashRpcMode::Unix => "http://localhost/",
            ZcashRpcMode::Tcp => self.config.rpc_url.as_str(),
        };

        let response = self
            .client
            .post(endpoint)
            .basic_auth(&self.config.rpc_user, Some(&self.config.rpc_password))
            .json(&request)
            .send()
            .await
            .context("zcash rpc request failed")?;

        let status = response.status();
        let payload = response
            .json::<RpcResponse>()
            .await
            .context("failed to decode zcash rpc response")?;

        if !status.is_success() {
            anyhow::bail!("zcash rpc returned non-success status: {status}");
        }

        if let Some(error) = payload.error {
            anyhow::bail!("zcash rpc error {}: {}", error.code, error.message);
        }

        payload.result.context("zcash rpc response missing result")
    }
}

pub fn validate_runtime_network_policy(config: &AppConfig) -> Result<()> {
    let is_dev_or_staging = matches!(config.app_env, AppEnv::Dev | AppEnv::Staging);
    let is_testnet = matches!(config.zcash_network, crate::config::ZcashNetwork::Testnet);
    if is_dev_or_staging && !is_testnet {
        anyhow::bail!("dev and staging environments must run with ZCASH_NETWORK=testnet");
    }
    Ok(())
}

pub fn validate_rpc_socket_policy(config: &AppConfig) -> Result<()> {
    if matches!(config.app_env, AppEnv::Prod)
        && !matches!(config.zcash_rpc_mode, ConfigRpcMode::Unix)
    {
        anyhow::bail!("production must use ZCASH_RPC_MODE=unix where possible");
    }
    Ok(())
}

pub fn evaluate_received_notes(notes: &[ReceivedNote], expected: Decimal) -> PaymentMatch {
    let total_received = notes
        .iter()
        .fold(Decimal::ZERO, |acc, note| acc + note.amount);
    let status = if total_received < expected {
        PaymentMatchStatus::Underpaid
    } else if total_received > expected {
        PaymentMatchStatus::Overpaid
    } else {
        PaymentMatchStatus::Exact
    };

    PaymentMatch {
        status,
        total_received,
        expected,
        note_count: notes.len(),
    }
}

fn parse_received_note(value: &Value) -> Result<ReceivedNote> {
    let txid = value
        .get("txid")
        .and_then(Value::as_str)
        .context("missing txid")?
        .to_owned();
    let address = value
        .get("address")
        .and_then(Value::as_str)
        .context("missing address")?
        .to_owned();
    let amount = parse_decimal_value(
        value
            .get("amount")
            .context("missing amount in received note")?,
    )?;
    let confirmations = value
        .get("confirmations")
        .and_then(Value::as_u64)
        .context("missing confirmations")?;
    let memo = value
        .get("memo")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Ok(ReceivedNote {
        txid,
        address,
        amount,
        confirmations,
        memo,
    })
}

fn parse_decimal_value(value: &Value) -> Result<Decimal> {
    if let Some(text) = value.as_str() {
        return text
            .parse::<Decimal>()
            .context("failed to parse decimal string");
    }
    if let Some(number) = value.as_f64() {
        return Decimal::from_f64_retain(number).context("failed to convert numeric amount");
    }
    anyhow::bail!("unsupported decimal value type")
}

#[cfg(test)]
mod tests {
    use super::{PaymentMatchStatus, ReceivedNote, evaluate_received_notes, parse_decimal_value};
    use rust_decimal::Decimal;
    use serde_json::json;

    #[test]
    fn parse_decimal_from_string_and_number() {
        let from_text = parse_decimal_value(&json!("1.25000000"));
        assert!(from_text.is_ok());
        assert_eq!(from_text.unwrap_or(Decimal::ZERO), Decimal::new(125, 2));

        let from_number = parse_decimal_value(&json!(2.75));
        assert!(from_number.is_ok());
        assert_eq!(from_number.unwrap_or(Decimal::ZERO), Decimal::new(275, 2));
    }

    #[test]
    fn evaluates_partial_exact_and_over_payments() {
        let notes = vec![
            ReceivedNote {
                txid: String::from("tx-1"),
                address: String::from("ztestsapling1abc"),
                amount: Decimal::new(10, 1),
                confirmations: 4,
                memo: None,
            },
            ReceivedNote {
                txid: String::from("tx-2"),
                address: String::from("ztestsapling1abc"),
                amount: Decimal::new(15, 1),
                confirmations: 3,
                memo: None,
            },
        ];

        let underpaid = evaluate_received_notes(&notes, Decimal::new(30, 1));
        assert_eq!(underpaid.status, PaymentMatchStatus::Underpaid);

        let exact = evaluate_received_notes(&notes, Decimal::new(25, 1));
        assert_eq!(exact.status, PaymentMatchStatus::Exact);

        let overpaid = evaluate_received_notes(&notes, Decimal::new(20, 1));
        assert_eq!(overpaid.status, PaymentMatchStatus::Overpaid);
    }
}
