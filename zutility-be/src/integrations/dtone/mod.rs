use std::{collections::BTreeMap, net::IpAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::http::{HeaderMap, header::AUTHORIZATION};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use rust_decimal::Decimal;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Map, Value, json};
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

pub const PROVIDER_NAME: &str = "dtone";
pub const CALLBACK_PATH: &str = "/api/v1/webhooks/dtone";
pub const EXTERNAL_ID_ALREADY_USED: i64 = 1_007_001;

const DEFAULT_BASE_URL: &str = "https://dvs-api.dtone.com/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const PRODUCT_PAGE_SIZE: u32 = 100;
const MAX_PRODUCT_PAGES: u32 = 100;
const RANGED_AMOUNT_KIND: &str = "ranged";
const IDENTIFIER_OBJECTS: [&str; 5] = [
    "credit_party_identifier",
    "beneficiary",
    "sender",
    "debit_party_identifier",
    "statement_identifier",
];

#[derive(Debug, Clone, Copy)]
pub struct DtoneCircuitBreakerPolicy {
    pub failure_threshold: u32,
    pub cooldown: Duration,
}

impl Default for DtoneCircuitBreakerPolicy {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DtoneCircuitBreaker {
    policy: DtoneCircuitBreakerPolicy,
    state: Arc<Mutex<DtoneCircuitBreakerState>>,
}

#[derive(Debug, Clone)]
struct DtoneCircuitBreakerState {
    consecutive_failures: u32,
    open_until: Option<chrono::DateTime<chrono::Utc>>,
    half_open_in_flight: bool,
}

impl DtoneCircuitBreaker {
    pub fn new(policy: DtoneCircuitBreakerPolicy) -> Self {
        Self {
            policy,
            state: Arc::new(Mutex::new(DtoneCircuitBreakerState {
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
        let reopen_after = |duration: &Duration| {
            chrono::Duration::from_std(*duration).unwrap_or_else(|_| chrono::Duration::seconds(30))
        };
        let was_probe = state.half_open_in_flight;
        state.half_open_in_flight = false;

        if was_probe {
            state.consecutive_failures = 0;
            state.open_until = Some(Utc::now() + reopen_after(&self.policy.cooldown));
            return;
        }

        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= self.policy.failure_threshold {
            state.consecutive_failures = 0;
            state.open_until = Some(Utc::now() + reopen_after(&self.policy.cooldown));
        }
    }
}

#[derive(Debug, Clone)]
pub struct DtoneSettings {
    pub base_url: String,
    pub api_key: SecretString,
    pub api_secret: SecretString,
    pub callback_url: String,
    pub callback_user: SecretString,
    pub callback_pass: SecretString,
    pub callback_ips: Vec<IpAddr>,
}

#[derive(Debug, Clone)]
pub struct DtoneProduct {
    pub id: String,
    pub name: String,
    pub country_iso3: String,
    pub service_id: String,
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DtoneBalance {
    pub available: Decimal,
    pub holding: Decimal,
    pub credit_limit: Decimal,
}

#[derive(Debug, Clone)]
pub struct DtoneBalanceRow {
    pub unit: String,
    pub balance: DtoneBalance,
}

#[derive(Debug, Clone)]
pub struct DtoneCallback {
    pub external_id: String,
    pub provider_transaction_id: String,
    pub class_id: i64,
    pub status: ProviderTxnStatus,
}

#[derive(Debug, Clone)]
pub struct DtoneTransaction {
    pub provider_transaction_id: String,
    pub external_id: Option<String>,
    pub class_id: Option<i64>,
    pub status: ProviderTxnStatus,
    pub token: Option<String>,
    pub raw: Value,
}

#[derive(Debug)]
enum DtoneCall {
    Provider(ProviderError),
    ExternalIdInUse,
}

impl From<ProviderError> for DtoneCall {
    fn from(error: ProviderError) -> Self {
        Self::Provider(error)
    }
}

impl DtoneCall {
    fn into_provider(self) -> ProviderError {
        match self {
            Self::Provider(error) => error,
            Self::ExternalIdInUse => ProviderError::new(
                ProviderErrorKind::Permanent,
                "dtone reported an external id conflict that could not be resolved by requery",
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DtoneClient {
    base_url: String,
    api_key: SecretString,
    api_secret: SecretString,
    callback_url: String,
    callback_user: SecretString,
    callback_pass: SecretString,
    callback_ips: Vec<IpAddr>,
    breaker: DtoneCircuitBreaker,
    client: Client,
}

impl DtoneClient {
    pub fn new(settings: DtoneSettings, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build dtone client")?;

        Ok(Self {
            base_url: settings.base_url,
            api_key: settings.api_key,
            api_secret: settings.api_secret,
            callback_url: settings.callback_url,
            callback_user: settings.callback_user,
            callback_pass: settings.callback_pass,
            callback_ips: settings.callback_ips,
            breaker: DtoneCircuitBreaker::new(DtoneCircuitBreakerPolicy::default()),
            client,
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let api_key = config
            .dtone_api_key
            .clone()
            .context("DTONE_API_KEY is required")?;
        let api_secret = config
            .dtone_api_secret
            .clone()
            .context("DTONE_API_SECRET is required when DTONE_API_KEY is set")?;
        let callback_url = config
            .dtone_callback_url
            .clone()
            .context("DTONE_CALLBACK_URL is required when DTONE_API_KEY is set")?;
        let callback_user = config
            .dtone_callback_user
            .clone()
            .context("DTONE_CALLBACK_USER is required when DTONE_API_KEY is set")?;
        let callback_pass = config
            .dtone_callback_pass
            .clone()
            .context("DTONE_CALLBACK_PASS is required when DTONE_API_KEY is set")?;
        let callback_ips = parse_callback_ips(config.dtone_callback_ips.as_deref())?;
        if callback_ips.is_empty() {
            tracing::warn!("DTONE_CALLBACK_IPS is empty, every DT One callback will be rejected");
        }

        Self::new(
            DtoneSettings {
                base_url: config
                    .dtone_base_url
                    .clone()
                    .unwrap_or_else(|| String::from(DEFAULT_BASE_URL)),
                api_key,
                api_secret,
                callback_url,
                callback_user,
                callback_pass,
                callback_ips,
            },
            DEFAULT_TIMEOUT,
        )
    }

    pub fn callback_ips(&self) -> &[IpAddr] {
        &self.callback_ips
    }

    pub fn is_allowed_callback_ip(&self, address: IpAddr) -> bool {
        self.callback_ips.contains(&address)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value, DtoneCall> {
        if !self.breaker.can_execute().await {
            return Err(DtoneCall::Provider(ProviderError::outage(
                "dtone circuit breaker is open",
            )));
        }

        let mut request = self
            .client
            .request(method, self.url(path))
            .basic_auth(
                self.api_key.expose_secret(),
                Some(self.api_secret.expose_secret()),
            )
            .header("Accept", "application/json");
        if !query.is_empty() {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .json(body);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                self.breaker.on_failure().await;
                return Err(DtoneCall::Provider(ProviderError::transient(format!(
                    "dtone request failed: {error}"
                ))));
            }
        };

        let status = response.status();
        let payload = match response.json::<Value>().await {
            Ok(payload) => payload,
            Err(_) if status.is_success() => {
                self.breaker.on_success().await;
                return Err(DtoneCall::Provider(ProviderError::new(
                    ProviderErrorKind::InvalidResponse,
                    "dtone returned a body that is not json",
                )));
            }
            Err(_) => Value::Null,
        };

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            self.breaker.on_success().await;
            return Err(DtoneCall::Provider(ProviderError::new(
                ProviderErrorKind::Unauthorized,
                "dtone authentication failed",
            )));
        }

        if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
            self.breaker.on_failure().await;
            return Err(DtoneCall::Provider(ProviderError::transient(format!(
                "dtone temporary failure with status {status}"
            ))));
        }

        if !status.is_success() {
            let codes = error_codes(&payload);
            self.breaker.on_success().await;
            if codes.contains(&EXTERNAL_ID_ALREADY_USED) {
                return Err(DtoneCall::ExternalIdInUse);
            }
            return Err(DtoneCall::Provider(ProviderError::permanent(format!(
                "dtone rejected the request with status {status} and error codes {codes:?}"
            ))));
        }

        self.breaker.on_success().await;
        Ok(payload)
    }

    pub async fn products(
        &self,
        country_iso3: &str,
        service_id: u32,
    ) -> Result<Vec<DtoneProduct>, ProviderError> {
        let mut products = Vec::new();
        let mut page = 1u32;

        loop {
            let query = [
                (String::from("country_iso_code"), country_iso3.to_owned()),
                (String::from("service_id"), service_id.to_string()),
                (String::from("page"), page.to_string()),
                (String::from("per_page"), PRODUCT_PAGE_SIZE.to_string()),
            ];
            let query: Vec<(&str, String)> = query
                .iter()
                .map(|(key, value)| (key.as_str(), value.clone()))
                .collect();

            let payload = self
                .send(Method::GET, "/products", &query, None)
                .await
                .map_err(DtoneCall::into_provider)?;

            let batch = rows(&payload);
            let fetched = u32::try_from(batch.len()).unwrap_or(PRODUCT_PAGE_SIZE);
            for row in batch {
                products.push(product_from_value(row, country_iso3, service_id));
            }

            if fetched < PRODUCT_PAGE_SIZE || page >= MAX_PRODUCT_PAGES {
                return Ok(products);
            }
            page += 1;
        }
    }

    pub async fn balance(&self) -> Result<Vec<DtoneBalanceRow>, ProviderError> {
        let payload = self
            .send(Method::GET, "/balances", &[], None)
            .await
            .map_err(DtoneCall::into_provider)?;

        Ok(rows(&payload)
            .into_iter()
            .map(|row| DtoneBalanceRow {
                unit: row
                    .get("unit")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                balance: DtoneBalance {
                    available: decimal_field(row, "available"),
                    holding: decimal_field(row, "holding"),
                    credit_limit: decimal_field(row, "credit_limit"),
                },
            })
            .collect())
    }

    pub async fn create_transaction(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        match self.create_async_transaction(request).await {
            Ok(payload) => Ok(purchase_response(request, &payload)),
            Err(DtoneCall::ExternalIdInUse) => {
                tracing::warn!(
                    order_id = %request.order_id,
                    external_id = %request.external_id,
                    "dtone external id already used, requerying instead of failing"
                );
                let payload = self
                    .transaction_by_external_id(&request.external_id)
                    .await?
                    .ok_or_else(|| {
                        ProviderError::new(
                            ProviderErrorKind::Permanent,
                            "dtone reported an external id conflict that could not be resolved by requery",
                        )
                    })?;
                Ok(purchase_response(request, &payload))
            }
            Err(DtoneCall::Provider(error)) => Err(error),
        }
    }

    async fn create_async_transaction(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<Value, DtoneCall> {
        let product_id = request.product_id.trim().parse::<i64>().map_err(|_| {
            DtoneCall::Provider(ProviderError::permanent("dtone product id is not numeric"))
        })?;

        let mut payload = Map::new();
        payload.insert(
            String::from("external_id"),
            Value::String(request.external_id.clone()),
        );
        payload.insert(String::from("product_id"), Value::Number(product_id.into()));
        payload.insert(String::from("auto_confirm"), Value::Bool(true));
        payload.insert(
            String::from("callback_url"),
            Value::String(self.callback_url.clone()),
        );

        for (key, value) in
            build_identifiers(&request.inputs, request.metadata.get("required_fields"))?
        {
            payload.insert(key, value);
        }

        if amount_kind(request) == RANGED_AMOUNT_KIND {
            let amount = request.settlement.amount().normalize();
            let amount = amount
                .to_string()
                .parse::<serde_json::Number>()
                .map_err(|_| {
                    DtoneCall::Provider(ProviderError::permanent(
                        "dtone settlement amount is not serialisable",
                    ))
                })?;
            payload.insert(
                String::from("calculation_mode"),
                Value::String(String::from("DESTINATION_AMOUNT")),
            );
            payload.insert(
                String::from("destination"),
                json!({ "amount": Value::Number(amount) }),
            );
        }

        self.send(
            Method::POST,
            "/async/transactions",
            &[],
            Some(&Value::Object(payload)),
        )
        .await
    }

    pub async fn transaction_by_external_id(
        &self,
        external_id: &str,
    ) -> Result<Option<Value>, ProviderError> {
        let query = [(String::from("external_id"), external_id.to_owned())];
        let query: Vec<(&str, String)> = query
            .iter()
            .map(|(key, value)| (key.as_str(), value.clone()))
            .collect();
        let payload = self
            .send(Method::GET, "/transactions", &query, None)
            .await
            .map_err(DtoneCall::into_provider)?;

        Ok(rows(&payload).into_iter().next().cloned())
    }

    pub async fn transaction_by_id(&self, id: &str) -> Result<Value, ProviderError> {
        self.send(
            Method::GET,
            &format!("/transactions/{}", id.trim()),
            &[],
            None,
        )
        .await
        .map_err(DtoneCall::into_provider)
    }

    pub async fn requery_transaction(
        &self,
        external_id: &str,
    ) -> Result<Option<DtoneTransaction>, ProviderError> {
        let payload = self.transaction_by_external_id(external_id).await?;
        Ok(payload.map(|value| transaction_from_value(&value)))
    }

    pub fn parse_callback(&self, body: &[u8]) -> Result<DtoneCallback, ProviderError> {
        let payload = serde_json::from_slice::<Value>(body).map_err(|_| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "dtone callback body is not json",
            )
        })?;
        let transaction = transaction_from_value(&payload);

        match (
            transaction.external_id,
            transaction.provider_transaction_id.is_empty(),
            transaction.class_id,
        ) {
            (Some(external_id), false, Some(class_id)) => Ok(DtoneCallback {
                external_id,
                provider_transaction_id: transaction.provider_transaction_id,
                class_id,
                status: transaction.status,
            }),
            _ => Err(ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "dtone callback is missing the transaction id, external id, or status class",
            )),
        }
    }
}

#[async_trait]
impl UtilityProvider for DtoneClient {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Dtone
    }

    async fn service_variations(
        &self,
        _service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError> {
        Err(ProviderError::permanent(
            "dtone products come from the catalogue, not a variations endpoint",
        ))
    }

    async fn validate_reference(
        &self,
        _request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError> {
        Ok(ValidateRefResponse {
            is_valid: true,
            customer_name: None,
            settlement: None,
            biller_name: None,
            product_name: None,
            reference: None,
            raw: json!({"provider": PROVIDER_NAME, "validation": "not required"}),
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        self.create_transaction(request).await
    }

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError> {
        let transaction = self.requery_transaction(request_id).await?.ok_or_else(|| {
            ProviderError::transient("dtone returned no transaction for the external id")
        })?;

        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status: transaction.status,
            token: transaction.token,
            raw: transaction.raw,
        })
    }

    fn verify_webhook(&self, headers: &HeaderMap, _body: &[u8]) -> bool {
        let Some(value) = headers.get(AUTHORIZATION) else {
            return false;
        };
        let Ok(value) = value.to_str() else {
            return false;
        };
        let Some((scheme, encoded)) = value.split_once(' ') else {
            return false;
        };
        if !scheme.eq_ignore_ascii_case("Basic") {
            return false;
        }
        let Ok(decoded) = BASE64.decode(encoded.trim()) else {
            return false;
        };
        let Ok(decoded) = String::from_utf8(decoded) else {
            return false;
        };
        let Some((user, password)) = decoded.split_once(':') else {
            return false;
        };

        let user_match = user
            .as_bytes()
            .ct_eq(self.callback_user.expose_secret().as_bytes());
        let password_match = password
            .as_bytes()
            .ct_eq(self.callback_pass.expose_secret().as_bytes());
        bool::from(user_match & password_match)
    }

    fn parse_webhook_event(&self, payload: &[u8]) -> Result<ProviderWebhookEvent, ProviderError> {
        let callback = self.parse_callback(payload)?;
        Ok(ProviderWebhookEvent {
            provider_request_id: callback.external_id,
            status: callback.status,
            token: None,
            raw: serde_json::from_slice(payload).unwrap_or(Value::Null),
        })
    }
}

fn parse_callback_ips(raw: Option<&str>) -> Result<Vec<IpAddr>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };

    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<IpAddr>()
                .with_context(|| format!("DTONE_CALLBACK_IPS contains an invalid address: {entry}"))
        })
        .collect()
}

fn rows(payload: &Value) -> Vec<&Value> {
    if let Some(entries) = payload.as_array() {
        return entries.iter().collect();
    }
    for key in ["data", "items", "results"] {
        if let Some(entries) = payload.get(key).and_then(Value::as_array) {
            return entries.iter().collect();
        }
    }
    Vec::new()
}

fn value_as_i64(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_str()
            .and_then(|raw| raw.trim().parse::<i64>().ok())
    })
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn error_codes(payload: &Value) -> Vec<i64> {
    let mut codes = Vec::new();
    if let Some(errors) = payload.get("errors").and_then(Value::as_array) {
        for entry in errors {
            if let Some(code) = entry.get("code").and_then(value_as_i64) {
                codes.push(code);
            }
        }
    }
    if let Some(code) = payload.get("code").and_then(value_as_i64) {
        codes.push(code);
    }
    codes
}

fn decimal_from_value(value: &Value) -> Option<Decimal> {
    match value {
        Value::Number(number) => number.to_string().parse::<Decimal>().ok(),
        Value::String(text) => text.trim().parse::<Decimal>().ok(),
        _ => None,
    }
}

fn decimal_field(value: &Value, key: &str) -> Decimal {
    value
        .get(key)
        .and_then(decimal_from_value)
        .unwrap_or(Decimal::ZERO)
}

pub fn status_class_id(payload: &Value) -> Option<i64> {
    let status = payload.get("status")?;
    status
        .get("class")
        .and_then(|class| class.get("id"))
        .and_then(value_as_i64)
        .or_else(|| status.get("id").and_then(value_as_i64))
}

pub fn map_status_class(class_id: i64) -> ProviderTxnStatus {
    match class_id / 10_000 {
        7 => ProviderTxnStatus::Delivered,
        8 => ProviderTxnStatus::Reversed,
        3 | 4 | 9 => ProviderTxnStatus::Failed,
        _ => ProviderTxnStatus::Pending,
    }
}

fn status_and_class(payload: &Value) -> (ProviderTxnStatus, Option<i64>) {
    match status_class_id(payload) {
        Some(class_id) => (map_status_class(class_id), Some(class_id)),
        None => (ProviderTxnStatus::Pending, None),
    }
}

fn pin_token(payload: &Value) -> Option<String> {
    payload
        .get("pin")
        .and_then(|pin| pin.get("code"))
        .and_then(value_as_string)
}

pub fn transaction_from_value(payload: &Value) -> DtoneTransaction {
    let (status, class_id) = status_and_class(payload);
    DtoneTransaction {
        provider_transaction_id: payload
            .get("id")
            .and_then(value_as_string)
            .unwrap_or_default(),
        external_id: payload
            .get("external_id")
            .and_then(value_as_string)
            .filter(|value| !value.is_empty()),
        class_id,
        status,
        token: pin_token(payload),
        raw: payload.clone(),
    }
}

fn product_from_value(payload: &Value, country_iso3: &str, service_id: u32) -> DtoneProduct {
    DtoneProduct {
        id: payload
            .get("id")
            .and_then(value_as_string)
            .unwrap_or_default(),
        name: payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        country_iso3: country_iso3.to_owned(),
        service_id: service_id.to_string(),
        raw: payload.clone(),
    }
}

fn purchase_response(request: &UtilityPurchaseRequest, payload: &Value) -> UtilityPurchaseResponse {
    let transaction = transaction_from_value(payload);
    UtilityPurchaseResponse {
        provider_reference: if transaction.provider_transaction_id.is_empty() {
            request.external_id.clone()
        } else {
            transaction.provider_transaction_id
        },
        provider_request_id: request.external_id.clone(),
        status: transaction.status,
        token: transaction.token,
        raw: payload.clone(),
    }
}

fn amount_kind(request: &UtilityPurchaseRequest) -> &str {
    request
        .metadata
        .get("amount_kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn build_identifiers(
    inputs: &BTreeMap<String, String>,
    required_fields: Option<&Value>,
) -> Result<Map<String, Value>, DtoneCall> {
    let mut body = Map::new();

    let Some(fields) = required_fields else {
        if !inputs.is_empty() {
            let mut identifier = Map::new();
            for (key, value) in inputs {
                identifier.insert(key.clone(), Value::String(value.clone()));
            }
            body.insert(
                String::from("credit_party_identifier"),
                Value::Object(identifier),
            );
        }
        return Ok(body);
    };

    let fields = fields.as_object().ok_or_else(|| {
        DtoneCall::Provider(ProviderError::new(
            ProviderErrorKind::InvalidResponse,
            "dtone required fields are malformed",
        ))
    })?;

    for object_key in IDENTIFIER_OBJECTS {
        let Some(entries) = fields.get(object_key) else {
            continue;
        };
        let entries = entries.as_array().ok_or_else(|| {
            DtoneCall::Provider(ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "dtone required identifier fields are malformed",
            ))
        })?;

        let mut identifier = Map::new();
        for entry in entries {
            let Some(name) = entry.as_str() else {
                continue;
            };
            let value = inputs.get(name).ok_or_else(|| {
                DtoneCall::Provider(ProviderError::permanent(format!(
                    "dtone order is missing the required identifier {name}"
                )))
            })?;
            identifier.insert(name.to_owned(), Value::String(value.clone()));
        }
        if !identifier.is_empty() {
            body.insert(object_key.to_owned(), Value::Object(identifier));
        }
    }

    if let Some(entries) = fields
        .get("additional_identifier")
        .and_then(Value::as_array)
        && let Some(name) = entries.first().and_then(Value::as_str)
    {
        let value = inputs.get(name).ok_or_else(|| {
            DtoneCall::Provider(ProviderError::permanent(format!(
                "dtone order is missing the required identifier {name}"
            )))
        })?;
        body.insert(
            String::from("additional_identifier"),
            json!({ "name": name, "value": value }),
        );
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::{map_status_class, status_class_id};
    use crate::integrations::utility_provider::ProviderTxnStatus;
    use serde_json::json;

    #[test]
    fn status_classes_map_by_five_digit_prefix() {
        assert_eq!(map_status_class(10000), ProviderTxnStatus::Pending);
        assert_eq!(map_status_class(20000), ProviderTxnStatus::Pending);
        assert_eq!(map_status_class(50000), ProviderTxnStatus::Pending);
        assert_eq!(map_status_class(70000), ProviderTxnStatus::Delivered);
        assert_eq!(map_status_class(30000), ProviderTxnStatus::Failed);
        assert_eq!(map_status_class(40000), ProviderTxnStatus::Failed);
        assert_eq!(map_status_class(90000), ProviderTxnStatus::Failed);
        assert_eq!(map_status_class(80000), ProviderTxnStatus::Reversed);
        assert_eq!(map_status_class(60000), ProviderTxnStatus::Pending);
    }

    #[test]
    fn status_class_reads_the_class_not_the_status_id() {
        let payload = json!({
            "status": {
                "class": {"id": 70000, "message": "COMPLETED"},
                "id": 70001,
                "message": "Completed"
            }
        });
        assert_eq!(status_class_id(&payload), Some(70000));
    }
}
