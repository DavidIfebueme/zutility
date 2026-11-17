use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::http::HeaderMap;
use chrono::{Datelike, Timelike, Utc};
use hmac::{Hmac, Mac};
use rand::{RngExt, distr::Alphanumeric};
use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    integrations::utility_provider::{
        ProviderError, ProviderErrorKind, ProviderKind, ProviderTxnStatus, ProviderWebhookEvent,
        RequeryResponse, UtilityProvider, UtilityPurchaseRequest, UtilityPurchaseResponse,
        UtilityVariation, ValidateRefRequest, ValidateRefResponse,
    },
};

fn cooldown(duration: &std::time::Duration) -> chrono::Duration {
    chrono::Duration::from_std(*duration).unwrap_or_else(|_| chrono::Duration::seconds(30))
}

#[derive(Debug, Clone)]
pub struct VtpassClient {
    base_url: String,
    api_key: SecretString,
    secret_key: SecretString,
    webhook_hmac_secret: SecretString,
    retry_policy: RetryPolicy,
    breaker: CircuitBreaker,
    client: Client,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
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
    half_open_in_flight: bool,
}

#[derive(Debug, Deserialize)]
struct VtpassStatusEnvelope {
    code: Option<String>,
    response_description: Option<String>,
    request_id: Option<String>,
    content: Option<Value>,
    token: Option<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(2),
            max_backoff: Duration::from_secs(12),
        }
    }
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
                half_open_in_flight: false,
            })),
        }
    }

    pub async fn can_execute(&self) -> bool {
        let mut state = self.state.lock().await;
        match state.open_until {
            Some(until) if Utc::now() >= until => {
                if state.half_open_in_flight {
                    false
                } else {
                    state.half_open_in_flight = true;
                    true
                }
            }
            Some(_) => false,
            None => true,
        }
    }

    pub async fn on_success(&self) {
        let mut state = self.state.lock().await;
        state.consecutive_failures = 0;
        state.open_until = None;
        state.half_open_in_flight = false;
    }

    pub async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        let was_probe = state.half_open_in_flight;
        state.half_open_in_flight = false;

        if was_probe {
            state.consecutive_failures = 0;
            state.open_until = Some(Utc::now() + cooldown(&self.policy.cooldown));
            return;
        }

        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.policy.failure_threshold {
            state.consecutive_failures = 0;
            state.open_until = Some(Utc::now() + cooldown(&self.policy.cooldown));
        }
    }
}

impl VtpassClient {
    pub fn new(
        base_url: String,
        api_key: SecretString,
        secret_key: SecretString,
        webhook_hmac_secret: SecretString,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build vtpass client")?;

        Ok(Self {
            base_url,
            api_key,
            secret_key,
            webhook_hmac_secret,
            retry_policy: RetryPolicy::default(),
            breaker: CircuitBreaker::new(CircuitBreakerPolicy::default()),
            client,
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        Self::new(
            config.vtpass_base_url.clone(),
            config.vtpass_api_key.clone(),
            config.vtpass_secret_key.clone(),
            config.signing_service_hmac_secret.clone(),
            Duration::from_millis(config.rate_source_timeout_ms),
        )
    }

    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    pub fn with_circuit_breaker_policy(mut self, policy: CircuitBreakerPolicy) -> Self {
        self.breaker = CircuitBreaker::new(policy);
        self
    }

    pub fn generate_request_id(&self) -> String {
        let lagos = Utc::now() + chrono::Duration::hours(1);
        let prefix = format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}",
            lagos.year(),
            lagos.month(),
            lagos.day(),
            lagos.hour(),
            lagos.minute(),
            lagos.second()
        );
        let suffix: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("{prefix}{suffix}")
    }

    pub fn request_id_for_order(&self, order_id: Uuid) -> String {
        let mut request_id = self.generate_request_id();
        let compact = order_id.as_simple().to_string();
        request_id.push_str(&compact[..12]);
        request_id
    }

    async fn post_json(&self, path: &str, payload: Value) -> Result<Value, ProviderError> {
        self.execute_with_retry(|| async {
            if !self.breaker.can_execute().await {
                return Err(ProviderError::outage("vtpass circuit breaker is open"));
            }

            let url = format!(
                "{}/{}",
                self.base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            );

            let response = self
                .client
                .post(url)
                .header("api-key", self.api_key.expose_secret())
                .header("secret-key", self.secret_key.expose_secret())
                .json(&payload)
                .send()
                .await
                .map_err(|error| {
                    ProviderError::transient(format!("vtpass request failed: {error}"))
                })?;

            let status = response.status();
            let response_json = response.json::<Value>().await.map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("vtpass json decode failed: {error}"),
                )
            })?;

            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                return Err(ProviderError::new(
                    ProviderErrorKind::Unauthorized,
                    "vtpass authentication failed",
                ));
            }

            if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                self.breaker.on_failure().await;
                return Err(ProviderError::transient(format!(
                    "vtpass temporary failure with status {status}"
                )));
            }

            if !status.is_success() {
                self.breaker.on_failure().await;
                return Err(ProviderError::permanent(format!(
                    "vtpass request rejected with status {status}"
                )));
            }

            self.breaker.on_success().await;
            Ok(response_json)
        })
        .await
    }

    async fn get_json(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, ProviderError> {
        self.execute_with_retry(|| async {
            if !self.breaker.can_execute().await {
                return Err(ProviderError::outage("vtpass circuit breaker is open"));
            }

            let url = format!(
                "{}/{}",
                self.base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            );
            let response = self
                .client
                .get(url)
                .header("api-key", self.api_key.expose_secret())
                .header("secret-key", self.secret_key.expose_secret())
                .query(query)
                .send()
                .await
                .map_err(|error| {
                    ProviderError::transient(format!("vtpass request failed: {error}"))
                })?;

            let status = response.status();
            let response_json = response.json::<Value>().await.map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("vtpass json decode failed: {error}"),
                )
            })?;

            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                return Err(ProviderError::new(
                    ProviderErrorKind::Unauthorized,
                    "vtpass authentication failed",
                ));
            }
            if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                self.breaker.on_failure().await;
                return Err(ProviderError::transient(format!(
                    "vtpass temporary failure with status {status}"
                )));
            }
            if !status.is_success() {
                self.breaker.on_failure().await;
                return Err(ProviderError::permanent(format!(
                    "vtpass request rejected with status {status}"
                )));
            }

            self.breaker.on_success().await;
            Ok(response_json)
        })
        .await
    }

    async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T, ProviderError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        let mut last_error: Option<ProviderError> = None;
        for attempt in 0..self.retry_policy.max_attempts {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(error)
                    if matches!(
                        error.kind,
                        ProviderErrorKind::Transient | ProviderErrorKind::Outage
                    ) && attempt + 1 < self.retry_policy.max_attempts =>
                {
                    last_error = Some(error);
                    let backoff = bounded_backoff(
                        self.retry_policy.initial_backoff,
                        self.retry_policy.max_backoff,
                        attempt,
                    );
                    tokio::time::sleep(backoff).await;
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::transient("vtpass retry exhausted")))
    }
}

#[async_trait]
impl UtilityProvider for VtpassClient {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Vtpass
    }

    async fn service_variations(
        &self,
        service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError> {
        let response = self
            .get_json("service-variations", &[("serviceID", service_id)])
            .await?;
        let content = response
            .get("content")
            .and_then(|value| value.get("variations"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    "service variations missing",
                )
            })?;

        Ok(content
            .iter()
            .map(|entry| UtilityVariation {
                variation_code: entry
                    .get("variation_code")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                amount: entry.get("variation_amount").and_then(Value::as_i64),
            })
            .collect())
    }

    async fn validate_reference(
        &self,
        request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError> {
        let response = self
            .post_json(
                "merchant-verify",
                json!({
                    "serviceID": request.service_id,
                    "billersCode": request.billers_code,
                }),
            )
            .await?;

        let envelope =
            serde_json::from_value::<VtpassStatusEnvelope>(response.clone()).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("vtpass validate response decode failed: {error}"),
                )
            })?;
        let is_valid = matches!(
            envelope.response_description.as_deref(),
            Some("TRANSACTION SUCCESSFUL") | Some("000")
        ) || envelope.code.as_deref() == Some("000");

        let customer_name = envelope
            .content
            .as_ref()
            .and_then(|value| value.get("Customer_Name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Ok(ValidateRefResponse {
            is_valid,
            customer_name,
            settlement: None,
            biller_name: None,
            product_name: None,
            reference: None,
            raw: response,
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let context = &request.context;
        let amount = request.settlement.to_ngn_whole().map_err(|error| {
            ProviderError::permanent(format!("vtpass amount is not whole naira: {error}"))
        })?;

        let response = self
            .post_json(
                "pay",
                json!({
                    "request_id": request.request_id,
                    "serviceID": context.service_id,
                    "billersCode": context.billers_code,
                    "variation_code": context.variation_code,
                    "amount": amount,
                    "phone": context.phone,
                    "metadata": request.metadata,
                    "order_id": request.order_id.to_string(),
                    "idempotency_ref": request.order_id.to_string(),
                    "zec_amount": request.zec_amount.to_string(),
                }),
            )
            .await?;

        let envelope =
            serde_json::from_value::<VtpassStatusEnvelope>(response.clone()).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("vtpass pay response decode failed: {error}"),
                )
            })?;
        let status = map_status(
            envelope.response_description.as_deref(),
            envelope.code.as_deref(),
        );
        let provider_reference = envelope
            .content
            .as_ref()
            .and_then(|value| value.get("transactionId"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        Ok(UtilityPurchaseResponse {
            provider_reference,
            provider_request_id: envelope
                .request_id
                .unwrap_or_else(|| request.request_id.clone()),
            status,
            token: envelope.token,
            raw: response,
        })
    }

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError> {
        let response = self
            .get_json("requery", &[("request_id", request_id)])
            .await?;

        let envelope =
            serde_json::from_value::<VtpassStatusEnvelope>(response.clone()).map_err(|error| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    format!("vtpass requery response decode failed: {error}"),
                )
            })?;

        Ok(RequeryResponse {
            provider_request_id: envelope.request_id.unwrap_or_else(|| request_id.to_owned()),
            status: map_status(
                envelope.response_description.as_deref(),
                envelope.code.as_deref(),
            ),
            token: envelope.token,
            raw: response,
        })
    }

    fn verify_webhook(&self, headers: &HeaderMap, body: &[u8]) -> bool {
        let Some(signature) = headers
            .get("x-vtpass-signature")
            .and_then(|value| value.to_str().ok())
        else {
            return false;
        };

        let Ok(mut mac) =
            Hmac::<Sha256>::new_from_slice(self.webhook_hmac_secret.expose_secret().as_bytes())
        else {
            return false;
        };
        mac.update(body);
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
            .get("request_id")
            .and_then(Value::as_str)
            .or_else(|| value.get("requestId").and_then(Value::as_str))
            .ok_or_else(|| {
                ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    "webhook request id missing",
                )
            })?
            .to_owned();

        let status = map_status(
            value.get("status").and_then(Value::as_str),
            value.get("code").and_then(Value::as_str),
        );
        let token = value
            .get("token")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Ok(ProviderWebhookEvent {
            provider_request_id: request_id,
            status,
            token,
            raw: value,
        })
    }
}

fn map_status(response_description: Option<&str>, code: Option<&str>) -> ProviderTxnStatus {
    match (response_description, code) {
        (Some("TRANSACTION SUCCESSFUL"), _) | (_, Some("000")) => ProviderTxnStatus::Delivered,
        (Some("PENDING"), _) | (_, Some("099")) => ProviderTxnStatus::Pending,
        _ => ProviderTxnStatus::Pending,
    }
}

fn bounded_backoff(initial: Duration, max: Duration, attempt: u8) -> Duration {
    let factor = 2_u64.saturating_pow(u32::from(attempt));
    let millis = initial
        .as_millis()
        .saturating_mul(u128::from(factor))
        .min(max.as_millis());
    Duration::from_millis(millis as u64)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::http::{HeaderMap, HeaderValue};
    use hmac::{Hmac, Mac};
    use secrecy::SecretString;
    use sha2::Sha256;
    use uuid::Uuid;

    use crate::integrations::utility_provider::UtilityProvider;

    use super::{CircuitBreaker, CircuitBreakerPolicy, VtpassClient, bounded_backoff};

    #[test]
    fn request_id_has_lagos_prefix_and_length() {
        let client = VtpassClient::new(
            String::from("https://sandbox.vtpass.com/api"),
            SecretString::from(String::from("api")),
            SecretString::from(String::from("secret")),
            SecretString::from(String::from("webhook")),
            Duration::from_secs(3),
        )
        .unwrap_or_else(|error| panic!("failed to construct vtpass client: {error}"));

        let request_id = client.generate_request_id();
        assert!(request_id.len() >= 20);
        assert!(
            request_id
                .chars()
                .take(14)
                .all(|char| char.is_ascii_digit())
        );

        let order_request_id = client.request_id_for_order(Uuid::new_v4());
        assert!(order_request_id.len() >= 26);
    }

    #[test]
    fn webhook_signature_verification_is_constant_time_match() {
        let client = VtpassClient::new(
            String::from("https://sandbox.vtpass.com/api"),
            SecretString::from(String::from("api")),
            SecretString::from(String::from("secret")),
            SecretString::from(String::from("webhook-secret")),
            Duration::from_secs(3),
        )
        .unwrap_or_else(|error| panic!("failed to construct vtpass client: {error}"));

        let payload = br#"{"request_id":"abc","status":"PENDING"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"webhook-secret")
            .unwrap_or_else(|_| panic!("failed to create hmac"));
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-vtpass-signature",
            HeaderValue::from_str(&signature).expect("signature header"),
        );

        assert!(client.verify_webhook(&headers, payload));

        let mut wrong = HeaderMap::new();
        wrong.insert(
            "x-vtpass-signature",
            HeaderValue::from_static("bad-signature"),
        );
        assert!(!client.verify_webhook(&wrong, payload));
        assert!(!client.verify_webhook(&HeaderMap::new(), payload));
    }

    #[test]
    fn bounded_backoff_stays_within_maximum() {
        let initial = Duration::from_secs(2);
        let max = Duration::from_secs(10);
        assert_eq!(bounded_backoff(initial, max, 0), Duration::from_secs(2));
        assert_eq!(bounded_backoff(initial, max, 1), Duration::from_secs(4));
        assert_eq!(bounded_backoff(initial, max, 2), Duration::from_secs(8));
        assert_eq!(bounded_backoff(initial, max, 3), Duration::from_secs(10));
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_failure_threshold() {
        let breaker = CircuitBreaker::new(CircuitBreakerPolicy {
            failure_threshold: 2,
            cooldown: Duration::from_secs(2),
        });
        assert!(breaker.can_execute().await);
        breaker.on_failure().await;
        assert!(breaker.can_execute().await);
        breaker.on_failure().await;
        assert!(!breaker.can_execute().await);
    }

    #[tokio::test]
    async fn circuit_breaker_allows_one_probe_and_reopens_on_failure() {
        let breaker = CircuitBreaker::new(CircuitBreakerPolicy {
            failure_threshold: 2,
            cooldown: Duration::from_millis(0),
        });

        breaker.on_failure().await;
        breaker.on_failure().await;
        assert!(breaker.can_execute().await);
        assert!(!breaker.can_execute().await);

        breaker.on_failure().await;
        assert!(breaker.can_execute().await);

        breaker.on_success().await;
        assert!(breaker.can_execute().await);
        assert!(breaker.can_execute().await);
    }
}
