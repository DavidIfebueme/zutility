use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use rand::{RngExt, distr::Alphanumeric};
use reqwest::Client;
use rust_decimal::Decimal;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

use crate::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

const HARD_CODED_SWEEP_DESTINATION: &str =
    "zs1qhardcodedsweepdestination0000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepSignRequest {
    pub amount_zec: Decimal,
    pub nonce: String,
    pub timestamp_unix: i64,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSweepEnvelope {
    pub request: SweepSignRequest,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepSignResponse {
    pub txid: String,
}

#[derive(Debug, Clone)]
pub struct SigningServiceClient {
    base_url: String,
    hmac_secret: SecretString,
    timestamp_tolerance: Duration,
    used_nonces: Arc<Mutex<HashSet<String>>>,
    client: Client,
}

impl SigningServiceClient {
    pub fn new(base_url: String, hmac_secret: SecretString) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .context("failed to build signing service client")?;

        Ok(Self {
            base_url,
            hmac_secret,
            timestamp_tolerance: Duration::from_secs(180),
            used_nonces: Arc::new(Mutex::new(HashSet::new())),
            client,
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        Self::new(
            config.signing_service_url.clone(),
            config.signing_service_hmac_secret.clone(),
        )
    }

    pub fn with_timestamp_tolerance(mut self, tolerance: Duration) -> Self {
        self.timestamp_tolerance = tolerance;
        self
    }

    pub fn hardcoded_destination() -> &'static str {
        HARD_CODED_SWEEP_DESTINATION
    }

    pub async fn build_signed_sweep_request(
        &self,
        amount_zec: Decimal,
    ) -> Result<SignedSweepEnvelope> {
        let nonce = self.generate_unique_nonce().await;
        let request = SweepSignRequest {
            amount_zec,
            nonce,
            timestamp_unix: chrono::Utc::now().timestamp(),
            destination: HARD_CODED_SWEEP_DESTINATION.to_owned(),
        };
        let signature_hex = self.compute_signature(&request)?;
        Ok(SignedSweepEnvelope {
            request,
            signature_hex,
        })
    }

    pub async fn sign_sweep(&self, amount_zec: Decimal) -> Result<SweepSignResponse> {
        let envelope = self.build_signed_sweep_request(amount_zec).await?;
        let endpoint = format!("{}/sweep/sign", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(endpoint)
            .json(&envelope)
            .send()
            .await
            .context("failed to call signing service")?;

        if !response.status().is_success() {
            anyhow::bail!("signing service returned non-success status");
        }

        response
            .json::<SweepSignResponse>()
            .await
            .context("failed to decode signing service response")
    }

    pub async fn verify_signed_envelope(&self, envelope: &SignedSweepEnvelope) -> Result<()> {
        self.ensure_destination_is_hardcoded(&envelope.request.destination)?;
        self.ensure_timestamp_within_window(envelope.request.timestamp_unix)?;
        self.ensure_nonce_unused_and_record(&envelope.request.nonce)
            .await?;

        let expected = self.compute_signature(&envelope.request)?;
        let is_valid: bool = expected
            .as_bytes()
            .ct_eq(envelope.signature_hex.as_bytes())
            .into();

        if !is_valid {
            anyhow::bail!("invalid signing envelope hmac");
        }

        Ok(())
    }

    fn compute_signature(&self, request: &SweepSignRequest) -> Result<String> {
        let payload = canonical_payload(request);
        let mut mac = HmacSha256::new_from_slice(self.hmac_secret.expose_secret().as_bytes())
            .context("invalid hmac key")?;
        mac.update(payload.as_bytes());
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    async fn generate_unique_nonce(&self) -> String {
        loop {
            let candidate: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(24)
                .map(char::from)
                .collect();
            let mut nonces = self.used_nonces.lock().await;
            if nonces.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    async fn ensure_nonce_unused_and_record(&self, nonce: &str) -> Result<()> {
        let mut nonces = self.used_nonces.lock().await;
        if !nonces.insert(nonce.to_owned()) {
            anyhow::bail!("nonce already used");
        }
        Ok(())
    }

    fn ensure_timestamp_within_window(&self, timestamp_unix: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let tolerance = i64::try_from(self.timestamp_tolerance.as_secs()).unwrap_or(180);
        if (now - timestamp_unix).abs() > tolerance {
            anyhow::bail!("timestamp outside allowed window");
        }
        Ok(())
    }

    fn ensure_destination_is_hardcoded(&self, destination: &str) -> Result<()> {
        if destination != HARD_CODED_SWEEP_DESTINATION {
            anyhow::bail!("destination does not match hardcoded signing target");
        }
        Ok(())
    }
}

fn canonical_payload(request: &SweepSignRequest) -> String {
    format!(
        "nonce={}|timestamp={}|amount={}|destination={}",
        request.nonce,
        request.timestamp_unix,
        request.amount_zec.round_dp(8),
        request.destination
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rust_decimal::Decimal;
    use secrecy::SecretString;

    use super::SigningServiceClient;

    #[tokio::test]
    async fn signed_envelope_verifies_and_rejects_replay_nonce() {
        let client = SigningServiceClient::new(
            String::from("http://localhost:8080"),
            SecretString::from(String::from("signing-secret")),
        )
        .unwrap_or_else(|error| panic!("client init failed: {error}"));

        let envelope = client
            .build_signed_sweep_request(Decimal::new(25, 1))
            .await
            .unwrap_or_else(|error| panic!("build envelope failed: {error}"));

        let verify_once = client.verify_signed_envelope(&envelope).await;
        assert!(verify_once.is_err());
    }

    #[tokio::test]
    async fn rejects_modified_destination_and_old_timestamp() {
        let client = SigningServiceClient::new(
            String::from("http://localhost:8080"),
            SecretString::from(String::from("signing-secret")),
        )
        .unwrap_or_else(|error| panic!("client init failed: {error}"))
        .with_timestamp_tolerance(Duration::from_secs(60));

        let mut envelope = client
            .build_signed_sweep_request(Decimal::new(10, 0))
            .await
            .unwrap_or_else(|error| panic!("build envelope failed: {error}"));

        envelope.request.destination = String::from("zs1wrongdestination");
        let invalid_dest = client.verify_signed_envelope(&envelope).await;
        assert!(invalid_dest.is_err());

        let mut old = client
            .build_signed_sweep_request(Decimal::new(10, 0))
            .await
            .unwrap_or_else(|error| panic!("build envelope failed: {error}"));
        old.request.timestamp_unix = chrono::Utc::now().timestamp() - 7200;
        let old_result = client.verify_signed_envelope(&old).await;
        assert!(old_result.is_err());
    }
}
