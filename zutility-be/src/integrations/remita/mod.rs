use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha512};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

use crate::{
    config::AppConfig,
    integrations::utility_provider::{
        ProviderError, ProviderErrorKind, ProviderKind, ProviderTxnStatus, ProviderWebhookEvent,
        RequeryResponse, UtilityProvider, UtilityPurchaseRequest, UtilityPurchaseResponse,
        UtilityVariation, ValidateRefRequest, ValidateRefResponse,
    },
};

#[derive(Debug, Clone)]
pub struct RemitaClient {
    base_url: String,
    merchant_id: String,
    api_key: SecretString,
    service_type_id: String,
    webhook_secret: SecretString,
    client: Client,
    breaker: CircuitBreaker,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerPolicy {
    pub failure_threshold: u32,
    pub cooldown: Duration,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    policy: CircuitBreakerPolicy,
    state: Arc<Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
struct CircuitBreakerState {
    consecutive_failures: u32,
    open_until: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for CircuitBreakerPolicy {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    pub fn new(policy: CircuitBreakerPolicy) -> Self {
        Self {
            policy,
            state: Arc::new(Mutex::new(CircuitBreakerState {
                consecutive_failures: 0,
                open_until: None,
            })),
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = self.state.lock().await;
        match state.open_until {
            Some(until) => chrono::Utc::now() >= until,
            None => true,
        }
    }

    pub async fn on_success(&self) {
        let mut state = self.state.lock().await;
        state.consecutive_failures = 0;
        state.open_until = None;
    }

    pub async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.policy.failure_threshold {
            let cooldown = chrono::Duration::from_std(self.policy.cooldown)
                .unwrap_or_else(|_| chrono::Duration::seconds(30));
            state.open_until = Some(chrono::Utc::now() + cooldown);
        }
    }
}

#[derive(Debug, Deserialize)]
struct RemitaStatusEnvelope {
    status_code: Option<String>,
    status: Option<String>,
    rrr: Option<String>,
    message: Option<String>,
}

impl RemitaClient {
    pub fn new(
        base_url: String,
        merchant_id: String,
        api_key: SecretString,
        service_type_id: String,
        webhook_secret: SecretString,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build remita client")?;

        Ok(Self {
            base_url,
            merchant_id,
            api_key,
            service_type_id,
            webhook_secret,
            client,
            breaker: CircuitBreaker::new(CircuitBreakerPolicy::default()),
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let base_url = config.remita_base_url.clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_BASE_URL not set"))?;
        let merchant_id = config.remita_merchant_id.clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_MERCHANT_ID not set"))?;
        let api_key = config.remita_api_key.clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_API_KEY not set"))?;
        let service_type_id = config.remita_service_type_id.clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_SERVICE_TYPE_ID not set"))?;
        let webhook_secret = config.remita_webhook_secret.clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_WEBHOOK_SECRET not set"))?;
        Self::new(
            base_url,
            merchant_id,
            api_key,
            service_type_id,
            webhook_secret,
            Duration::from_millis(config.rate_source_timeout_ms),
        )
    }

    pub fn compute_hash(&self, order_id: &str, amount_kobo: i64) -> String {
        let concat = format!(
            "{}{}{}{}{}",
            self.merchant_id,
            self.service_type_id,
            order_id,
            amount_kobo,
            self.api_key.expose_secret()
        );
        let result = Sha512::digest(concat.as_bytes());
        hex::encode(result)
    }

    async fn post_json(&self, path: &str, payload: Value) -> Result<Value, ProviderError> {
        if !self.breaker.can_execute().await {
            return Err(ProviderError::outage("remita circuit breaker is open"));
        }

        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("remita request failed: {error}"))
            })?;

        let status = response.status();
        let response_json = response.json::<Value>().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("remita json decode failed: {error}"),
            )
        })?;

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.breaker.on_failure().await;
            return Err(ProviderError::new(
                ProviderErrorKind::Unauthorized,
                "remita authentication failed",
            ));
        }

        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            self.breaker.on_failure().await;
            return Err(ProviderError::transient(format!(
                "remita temporary failure with status {status}"
            )));
        }

        if !status.is_success() {
            self.breaker.on_failure().await;
            return Err(ProviderError::permanent(format!(
                "remita request rejected with status {status}"
            )));
        }

        self.breaker.on_success().await;
        Ok(response_json)
    }

    async fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, ProviderError> {
        if !self.breaker.can_execute().await {
            return Err(ProviderError::outage("remita circuit breaker is open"));
        }

        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .client
            .get(url)
            .header("Content-Type", "application/json")
            .query(query)
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("remita request failed: {error}"))
            })?;

        let status = response.status();
        let response_json = response.json::<Value>().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("remita json decode failed: {error}"),
            )
        })?;

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.breaker.on_failure().await;
            return Err(ProviderError::new(
                ProviderErrorKind::Unauthorized,
                "remita authentication failed",
            ));
        }
        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            self.breaker.on_failure().await;
            return Err(ProviderError::transient(format!(
                "remita temporary failure with status {status}"
            )));
        }
        if !status.is_success() {
            self.breaker.on_failure().await;
            return Err(ProviderError::permanent(format!(
                "remita request rejected with status {status}"
            )));
        }

        self.breaker.on_success().await;
        Ok(response_json)
    }
}

#[async_trait]
impl UtilityProvider for RemitaClient {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Remita
    }

    async fn service_variations(
        &self,
        _service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_reference(
        &self,
        request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError> {
        if request.billers_code.trim().len() == 12 {
            let status = self
                .get_json(
                    &format!("rrr/{}/status", request.billers_code),
                    &[],
                )
                .await;

            match status {
                Ok(value) => {
                    let rrr = value.get("rrr").and_then(Value::as_str).map(ToOwned::to_owned);
                    let is_valid = rrr.is_some();
                    let customer_name = value
                        .get("payerName")
                        .or_else(|| value.get("payer_name"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    Ok(ValidateRefResponse {
                        is_valid,
                        customer_name,
                        raw: value,
                    })
                }
                Err(_) => Ok(ValidateRefResponse {
                    is_valid: false,
                    customer_name: None,
                    raw: json!({}),
                }),
            }
        } else {
            Ok(ValidateRefResponse {
                is_valid: true,
                customer_name: None,
                raw: json!({"billers_code": request.billers_code, "service_id": request.service_id}),
            })
        }
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let hash = self.compute_hash(&request.order_id.to_string(), request.amount_ngn);
        let payload = json!({
            "merchantId": self.merchant_id,
            "serviceTypeId": self.service_type_id,
            "orderId": request.order_id.to_string(),
            "amount": request.amount_ngn / 100,
            "payerName": request.metadata.get("customer_name").and_then(Value::as_str).unwrap_or("Zutility User"),
            "payerEmail": request.metadata.get("payer_email").and_then(Value::as_str).unwrap_or("noreply@zutility.xyz"),
            "payerPhone": request.metadata.get("payer_phone").and_then(Value::as_str).unwrap_or(&request.billers_code),
            "description": format!("Zutility: {} for {}", request.service_id, request.billers_code),
            "hash": hash,
        });

        let response = self.post_json("billpayment/pay", payload).await?;

        let envelope = serde_json::from_value::<RemitaStatusEnvelope>(response.clone())
            .map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("remita pay response decode failed: {error}"),
                )
            })?;

        let status_code = envelope.status_code.as_deref().unwrap_or("");
        let status = match status_code {
            "022" | "023" => ProviderTxnStatus::Pending,
            "021" | "00" | "01" => ProviderTxnStatus::Delivered,
            _ => ProviderTxnStatus::Failed,
        };

        Ok(UtilityPurchaseResponse {
            provider_reference: envelope.rrr.unwrap_or_default(),
            provider_request_id: request.order_id.to_string(),
            status,
            token: None,
            raw: response,
        })
    }

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError> {
        let hash = format!(
            "{}{}{}",
            self.merchant_id,
            request_id,
            self.api_key.expose_secret()
        );
        let result = Sha512::digest(hash.as_bytes());
        let hash_hex = hex::encode(result);

        let response = self
            .get_json(
                &format!("billpayment/status/{}/{}/{}", self.merchant_id, request_id, hash_hex),
                &[],
            )
            .await?;

        let envelope = serde_json::from_value::<RemitaStatusEnvelope>(response.clone())
            .map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("remita requery response decode failed: {error}"),
                )
            })?;

        let status_code = envelope.status_code.as_deref().unwrap_or("");
        let status = match status_code {
            "022" | "023" => ProviderTxnStatus::Pending,
            "021" | "00" | "01" => ProviderTxnStatus::Delivered,
            _ => ProviderTxnStatus::Failed,
        };

        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status,
            token: None,
            raw: response,
        })
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.webhook_secret.expose_secret().as_bytes())
            .unwrap_or_else(|_| panic!("invalid hmac key"));
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());
        expected
            .as_bytes()
            .ct_eq(signature.trim().as_bytes())
            .into()
    }

    fn parse_webhook_event(&self, payload: &[u8]) -> Result<ProviderWebhookEvent, ProviderError> {
        let value = serde_json::from_slice::<Value>(payload).map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("invalid webhook payload: {error}"),
            )
        })?;

        let request_id = value
            .get("orderId")
            .or_else(|| value.get("rrr"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    "webhook order id missing",
                )
            })?
            .to_owned();

        let status_code = value
            .get("statusCode")
            .and_then(Value::as_str)
            .unwrap_or("");

        let status = match status_code {
            "021" | "00" | "01" => ProviderTxnStatus::Delivered,
            "022" | "023" => ProviderTxnStatus::Pending,
            _ => ProviderTxnStatus::Failed,
        };

        Ok(ProviderWebhookEvent {
            provider_request_id: request_id,
            status,
            token: None,
            raw: value,
        })
    }
}
