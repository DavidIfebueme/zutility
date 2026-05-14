use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Datelike, Timelike, Utc};
use reqwest::{Client, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
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

#[derive(Debug, Clone, Copy)]
pub struct InlomaxCircuitBreakerPolicy {
    pub failure_threshold: u32,
    pub cooldown: Duration,
}

#[derive(Debug, Clone)]
pub struct InlomaxCircuitBreaker {
    policy: InlomaxCircuitBreakerPolicy,
    state: Arc<Mutex<InlomaxCircuitBreakerState>>,
}

#[derive(Debug, Clone)]
struct InlomaxCircuitBreakerState {
    consecutive_failures: u32,
    open_until: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for InlomaxCircuitBreakerPolicy {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

impl InlomaxCircuitBreaker {
    pub fn new(policy: InlomaxCircuitBreakerPolicy) -> Self {
        Self {
            policy,
            state: Arc::new(Mutex::new(InlomaxCircuitBreakerState {
                consecutive_failures: 0,
                open_until: None,
            })),
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = self.state.lock().await;
        match state.open_until {
            Some(until) => Utc::now() >= until,
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
            state.open_until = Some(Utc::now() + cooldown);
        }
    }
}

#[derive(Debug, Clone)]
pub struct InlomaxClient {
    base_url: String,
    api_key: SecretString,
    webhook_secret: SecretString,
    breaker: InlomaxCircuitBreaker,
    client: Client,
}

impl InlomaxClient {
    pub fn new(
        base_url: String,
        api_key: SecretString,
        webhook_secret: SecretString,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build inlomax client")?;

        Ok(Self {
            base_url,
            api_key,
            webhook_secret,
            breaker: InlomaxCircuitBreaker::new(InlomaxCircuitBreakerPolicy::default()),
            client,
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let base_url = config
            .inlomax_base_url
            .clone()
            .unwrap_or_else(|| String::from("https://inlomax.com/api"));
        let api_key = config
            .inlomax_api_key
            .clone()
            .context("INLOMAX_API_KEY is required")?;
        let webhook_secret = config
            .inlomax_webhook_secret
            .clone()
            .unwrap_or_else(|| SecretString::from(String::from("inlomax-webhook-default")));

        Self::new(
            base_url,
            api_key,
            webhook_secret,
            Duration::from_millis(config.rate_source_timeout_ms),
        )
    }

    pub fn request_id_for_order(&self, order_id: Uuid) -> String {
        let prefix = format!(
            "inl-{:04}{:02}{:02}{:02}{:02}{:02}",
            Utc::now().year(),
            Utc::now().month(),
            Utc::now().day(),
            Utc::now().hour(),
            Utc::now().minute(),
            Utc::now().second()
        );
        let compact = order_id.as_simple().to_string();
        format!("{prefix}-{}", &compact[..12])
    }

    async fn post_json(&self, path: &str, payload: Value) -> Result<Value, ProviderError> {
        if !self.breaker.can_execute().await {
            return Err(ProviderError::outage("inlomax circuit breaker is open"));
        }

        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key.expose_secret()))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("inlomax request failed: {error}"))
            })?;

        let status = response.status();
        let response_json = response.json::<Value>().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("inlomax json decode failed: {error}"),
            )
        })?;

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.breaker.on_failure().await;
            return Err(ProviderError::new(
                ProviderErrorKind::Unauthorized,
                "inlomax authentication failed",
            ));
        }

        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            self.breaker.on_failure().await;
            return Err(ProviderError::transient(format!(
                "inlomax temporary failure with status {status}"
            )));
        }

        if !status.is_success() {
            self.breaker.on_failure().await;
            return Err(ProviderError::permanent(format!(
                "inlomax request rejected with status {status}"
            )));
        }

        self.breaker.on_success().await;
        Ok(response_json)
    }

    async fn get_json(&self, path: &str) -> Result<Value, ProviderError> {
        if !self.breaker.can_execute().await {
            return Err(ProviderError::outage("inlomax circuit breaker is open"));
        }

        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Token {}", self.api_key.expose_secret()))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("inlomax request failed: {error}"))
            })?;

        let status = response.status();
        let response_json = response.json::<Value>().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("inlomax json decode failed: {error}"),
            )
        })?;

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.breaker.on_failure().await;
            return Err(ProviderError::new(
                ProviderErrorKind::Unauthorized,
                "inlomax authentication failed",
            ));
        }
        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            self.breaker.on_failure().await;
            return Err(ProviderError::transient(format!(
                "inlomax temporary failure with status {status}"
            )));
        }
        if !status.is_success() {
            self.breaker.on_failure().await;
            return Err(ProviderError::permanent(format!(
                "inlomax request rejected with status {status}"
            )));
        }

        self.breaker.on_success().await;
        Ok(response_json)
    }

    fn map_status(status_str: Option<&str>) -> ProviderTxnStatus {
        match status_str {
            Some("success") => ProviderTxnStatus::Delivered,
            Some("processing") => ProviderTxnStatus::Pending,
            Some("failed") => ProviderTxnStatus::Failed,
            _ => ProviderTxnStatus::Failed,
        }
    }
}

fn slug_to_inlomax_service_id(slug: &str) -> Option<String> {
    match slug {
        "mtn" => Some(String::from("1")),
        "airtel" => Some(String::from("2")),
        "glo" => Some(String::from("3")),
        "9mobile" => Some(String::from("4")),
        "mtn-data" => Some(String::from("1")),
        "airtel-data" => Some(String::from("2")),
        "glo-data" => Some(String::from("3")),
        "9mobile-data" => Some(String::from("4")),
        "dstv" => Some(String::from("90")),
        "gotv" => Some(String::from("95")),
        "startimes" => Some(String::from("101")),
        "ikeja-electric" => Some(String::from("1")),
        "eko-electric" => Some(String::from("2")),
        "abuja-electric" => Some(String::from("3")),
        "ibadan-electric" => Some(String::from("4")),
        "kano-electric" => Some(String::from("5")),
        "phed-electric" => Some(String::from("6")),
        "jos-electric" => Some(String::from("7")),
        "kaduna-electric" => Some(String::from("8")),
        "enugu-electric" => Some(String::from("9")),
        "benin-electric" => Some(String::from("10")),
        "yola-electric" => Some(String::from("11")),
        "aba-electric" => Some(String::from("12")),
        "waec-registration" | "waec-result-checker" => Some(String::from("1")),
        "jamb" => Some(String::from("2")),
        _ => None,
    }
}

fn utility_type_to_inlomax_endpoint(utility_type: &str) -> Option<&'static str> {
    match utility_type {
        "airtime" => Some("airtime"),
        "data" => Some("data"),
        "dstv" | "gotv" | "startimes" => Some("subcable"),
        "electricity" => Some("payelectric"),
        "waec" | "jamb" => Some("education"),
        _ => None,
    }
}

#[async_trait]
impl UtilityProvider for InlomaxClient {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Inlomax
    }

    async fn service_variations(
        &self,
        service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError> {
        let response = self.get_json("services").await?;
        let data = response.get("data").ok_or_else(|| {
            ProviderError::new(ProviderErrorKind::InvalidResponse, "inlomax services missing data field")
        })?;

        let mut variations = Vec::new();

        if let Some(airtime) = data.get("airtime").and_then(Value::as_array) {
            for entry in airtime {
                if entry.get("serviceID").and_then(Value::as_str) == Some(service_id) {
                    variations.push(UtilityVariation {
                        variation_code: entry.get("serviceID").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        name: entry.get("network").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        amount: None,
                    });
                }
            }
        }

        if let Some(data_plans) = data.get("dataPlans").and_then(Value::as_array) {
            for entry in data_plans {
                if entry.get("serviceID").and_then(Value::as_str) == Some(service_id) {
                    let plan_name = format!(
                        "{} - {} ({})",
                        entry.get("dataPlan").and_then(Value::as_str).unwrap_or_default(),
                        entry.get("amount").and_then(Value::as_str).unwrap_or_default(),
                        entry.get("dataType").and_then(Value::as_str).unwrap_or_default(),
                    );
                    let amount = entry.get("amount").and_then(|v| v.as_str()).and_then(|s| s.replace(',', "").parse::<i64>().ok());
                    variations.push(UtilityVariation {
                        variation_code: entry.get("serviceID").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        name: plan_name,
                        amount,
                    });
                }
            }
        }

        if let Some(cable_plans) = data.get("cablePlans").and_then(Value::as_array) {
            for entry in cable_plans {
                if entry.get("serviceID").and_then(Value::as_str) == Some(service_id) {
                    let amount = entry.get("amount").and_then(|v| v.as_str()).and_then(|s| s.replace(',', "").parse::<i64>().ok());
                    variations.push(UtilityVariation {
                        variation_code: entry.get("serviceID").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        name: entry.get("cablePlan").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        amount,
                    });
                }
            }
        }

        if let Some(education) = data.get("education").and_then(Value::as_array) {
            for entry in education {
                if entry.get("serviceID").and_then(Value::as_str) == Some(service_id) {
                    let amount = entry.get("amount").and_then(|v| v.as_str()).and_then(|s| s.replace(',', "").parse::<i64>().ok());
                    variations.push(UtilityVariation {
                        variation_code: entry.get("serviceID").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        name: entry.get("type").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        amount,
                    });
                }
            }
        }

        Ok(variations)
    }

    async fn validate_reference(
        &self,
        request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError> {
        let response = match request.service_id.as_str() {
            id if ["90", "95", "96", "97", "98", "99", "100", "101", "102", "103", "104", "105"].contains(&id) => {
                self.post_json(
                    "validatecable",
                    json!({
                        "serviceID": request.service_id,
                        "iucNum": request.billers_code,
                    }),
                )
                .await?
            }
            id if ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"].contains(&id) => {
                self.post_json(
                    "validatemeter",
                    json!({
                        "serviceID": request.service_id,
                        "meterNum": request.billers_code,
                        "meterType": 1,
                    }),
                )
                .await?
            }
            _ => {
                return Ok(ValidateRefResponse {
                    is_valid: true,
                    customer_name: None,
                    raw: serde_json::json!({"status": "success", "message": "no validation required"}),
                });
            }
        };

        let status = response.get("status").and_then(Value::as_str).unwrap_or_default();
        let is_valid = status == "success";
        let customer_name = response
            .get("data")
            .and_then(|d| d.get("customerName"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Ok(ValidateRefResponse {
            is_valid,
            customer_name,
            raw: response,
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let endpoint = utility_type_to_inlomax_endpoint(&request.metadata.get("utility_type").and_then(Value::as_str).unwrap_or(""))
            .ok_or_else(|| ProviderError::permanent("inlomax does not support this utility type"))?;

        let request_id = self.request_id_for_order(request.order_id);

        let payload = match request.metadata.get("utility_type").and_then(Value::as_str).unwrap_or("") {
            "airtime" => {
                let sid = slug_to_inlomax_service_id(&request.service_id)
                    .unwrap_or_else(|| request.service_id.clone());
                json!({
                    "serviceID": sid,
                    "amount": request.amount_ngn,
                    "mobileNumber": request.billers_code,
                    "request-id": request_id,
                })
            }
            "data" => {
                let sid = request.variation_code.as_deref()
                    .map(|v| slug_to_inlomax_service_id(v).unwrap_or_else(|| v.to_owned()))
                    .unwrap_or_else(|| slug_to_inlomax_service_id(&request.service_id)
                        .unwrap_or_else(|| request.service_id.clone()));
                json!({
                    "serviceID": sid,
                    "mobileNumber": request.billers_code,
                    "request-id": request_id,
                })
            }
            "dstv" | "gotv" | "startimes" => {
                let sid = request.variation_code.as_deref()
                    .map(|v| slug_to_inlomax_service_id(v).unwrap_or_else(|| v.to_owned()))
                    .unwrap_or_else(|| slug_to_inlomax_service_id(&request.service_id)
                        .unwrap_or_else(|| request.service_id.clone()));
                json!({
                    "serviceID": sid,
                    "iucNum": request.billers_code,
                    "request-id": request_id,
                })
            }
            "electricity" => {
                let sid = slug_to_inlomax_service_id(&request.service_id)
                    .unwrap_or_else(|| request.service_id.clone());
                json!({
                    "serviceID": sid,
                    "meterNum": request.billers_code,
                    "meterType": 1,
                    "amount": request.amount_ngn,
                    "request-id": request_id,
                })
            }
            "waec" | "jamb" => {
                let sid = slug_to_inlomax_service_id(&request.service_id)
                    .unwrap_or_else(|| request.service_id.clone());
                json!({
                    "serviceID": sid,
                    "quantity": 1,
                    "request-id": request_id,
                })
            }
            _ => return Err(ProviderError::permanent("unsupported utility type for inlomax")),
        };

        let response = self.post_json(endpoint, payload).await?;

        let status_str = response.get("status").and_then(Value::as_str).unwrap_or_default();
        let status = Self::map_status(Some(status_str));

        let data = response.get("data").cloned().unwrap_or(json!({}));
        let reference = data
            .get("reference")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let token = data
            .get("token")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Ok(UtilityPurchaseResponse {
            provider_reference: reference.clone(),
            provider_request_id: if reference.is_empty() { request_id } else { reference.clone() },
            status,
            token,
            raw: response,
        })
    }

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError> {
        let response = self
            .post_json("transaction", json!({"reference": request_id}))
            .await?;

        let data = response.get("data").cloned().unwrap_or(json!({}));
        let status_str = data.get("status").and_then(Value::as_str).unwrap_or_default();
        let status = Self::map_status(Some(status_str));
        let token = data.get("token").and_then(Value::as_str).map(ToOwned::to_owned);

        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status,
            token,
            raw: response,
        })
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(self.webhook_secret.expose_secret().as_bytes())
            .unwrap_or_else(|_| panic!("invalid hmac key"));
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());
        expected.as_bytes().ct_eq(signature.trim().as_bytes()).into()
    }

    fn parse_webhook_event(&self, payload: &[u8]) -> Result<ProviderWebhookEvent, ProviderError> {
        let value = serde_json::from_slice::<Value>(payload).map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("invalid inlomax webhook payload: {error}"),
            )
        })?;

        let reference = value
            .get("data")
            .and_then(|d| d.get("reference"))
            .and_then(Value::as_str)
            .or_else(|| value.get("reference").and_then(Value::as_str))
            .unwrap_or_default()
            .to_owned();

        let status_str = value
            .get("data")
            .and_then(|d| d.get("status"))
            .and_then(Value::as_str)
            .or_else(|| value.get("status").and_then(Value::as_str))
            .unwrap_or_default();

        let status = Self::map_status(Some(status_str));
        let token = value
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Ok(ProviderWebhookEvent {
            provider_request_id: reference,
            status,
            token,
            raw: value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_to_service_id_maps_correctly() {
        assert_eq!(slug_to_inlomax_service_id("mtn"), Some(String::from("1")));
        assert_eq!(slug_to_inlomax_service_id("airtel"), Some(String::from("2")));
        assert_eq!(slug_to_inlomax_service_id("ikeja-electric"), Some(String::from("1")));
        assert_eq!(slug_to_inlomax_service_id("unknown"), None);
    }

    #[test]
    fn status_mapping_works() {
        assert_eq!(InlomaxClient::map_status(Some("success")), ProviderTxnStatus::Delivered);
        assert_eq!(InlomaxClient::map_status(Some("processing")), ProviderTxnStatus::Pending);
        assert_eq!(InlomaxClient::map_status(Some("failed")), ProviderTxnStatus::Failed);
        assert_eq!(InlomaxClient::map_status(None), ProviderTxnStatus::Failed);
    }
}
