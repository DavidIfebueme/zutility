use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::http::HeaderMap;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    config::AppConfig,
    domain::money::Money,
    integrations::utility_provider::{
        ProviderError, ProviderErrorKind, ProviderKind, ProviderTxnStatus, ProviderWebhookEvent,
        RequeryResponse, UtilityProvider, UtilityPurchaseRequest, UtilityPurchaseResponse,
        UtilityVariation, ValidateRefRequest, ValidateRefResponse,
    },
};

#[derive(Debug, Clone)]
pub struct RemitaCollectionsClient {
    base_url: String,
    secret_key: SecretString,
    client: Client,
}

#[derive(Debug, Clone, Deserialize)]
struct RemitaEnvelope {
    status: String,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct RrrDetails {
    pub biller_id: String,
    pub biller_name: String,
    pub product_name: Option<String>,
    pub customer_name: Option<String>,
    pub amount_ngn: i64,
    pub fee_ngn: i64,
    pub rrr: String,
    pub status: String,
    pub receipt_url: Option<String>,
    pub description: Option<String>,
    pub raw: Value,
}

impl RrrDetails {
    pub fn is_payable(&self) -> bool {
        matches!(
            self.status.to_ascii_lowercase().as_str(),
            "pending" | "unpaid"
        )
    }

    pub fn transaction_status(&self) -> ProviderTxnStatus {
        map_transaction_status(&self.status)
    }
}

#[derive(Debug, Clone)]
pub struct BillerTransaction {
    pub rrr: Option<String>,
    pub status: String,
    pub raw: Value,
}

impl BillerTransaction {
    pub fn transaction_status(&self) -> ProviderTxnStatus {
        map_transaction_status(&self.status)
    }
}

fn map_transaction_status(status: &str) -> ProviderTxnStatus {
    match status.to_ascii_lowercase().as_str() {
        "paid" | "completed" | "successful" | "success" | "approved" => {
            ProviderTxnStatus::Delivered
        }
        "failed" | "cancelled" | "canceled" | "reversed" | "expired" | "declined" => {
            ProviderTxnStatus::Failed
        }
        _ => ProviderTxnStatus::Pending,
    }
}

impl RemitaCollectionsClient {
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let base_url = config
            .remita_collections_base_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_COLLECTIONS_BASE_URL not set"))?;
        let secret_key = config
            .remita_collections_secret_key
            .clone()
            .ok_or_else(|| anyhow::anyhow!("REMITA_COLLECTIONS_SECRET_KEY not set"))?;

        Self::new(base_url, secret_key, Duration::from_secs(30))
    }

    pub fn new(base_url: String, secret_key: SecretString, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("failed to build remita collections client")?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            secret_key,
            client,
        })
    }

    pub async fn lookup_rrr(&self, rrr: &str) -> Result<RrrDetails, ProviderError> {
        let url = format!("{}/biller/lookup/{}", self.base_url, rrr);
        let response = self
            .client
            .get(&url)
            .header("secretKey", self.secret_key.expose_secret())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("remita lookup request failed: {error}"))
            })?;

        let status = response.status();
        let envelope: RemitaEnvelope = response.json().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("remita lookup decode failed: {error}"),
            )
        })?;

        if !status.is_success() || envelope.status != "00" {
            return Err(envelope_error(&envelope));
        }

        let data = envelope.data.ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "remita lookup returned no data",
            )
        })?;

        parse_rrr_details(data)
    }

    pub async fn process_rrr(
        &self,
        rrr: &str,
        payment_identifier: &str,
        amount_ngn: i64,
    ) -> Result<ProviderTxnStatus, ProviderError> {
        let url = format!("{}/biller/process", self.base_url);
        let payload = json!({
            "rrr": rrr,
            "paymentIdentifier": payment_identifier,
            "amount": amount_ngn,
        });

        let response = self
            .client
            .post(&url)
            .header("secretKey", self.secret_key.expose_secret())
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("remita process request failed: {error}"))
            })?;

        let status = response.status();
        let envelope: RemitaEnvelope = response.json().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("remita process decode failed: {error}"),
            )
        })?;

        if !status.is_success() || envelope.status != "00" {
            return Err(envelope_error(&envelope));
        }

        let transaction_status = envelope
            .data
            .as_ref()
            .and_then(|data| data.get("transactionStatus"))
            .and_then(Value::as_str)
            .unwrap_or("PROCESSING");

        Ok(match transaction_status.to_ascii_lowercase().as_str() {
            "success" | "successful" | "completed" | "paid" | "approved" => {
                ProviderTxnStatus::Delivered
            }
            "failed" | "declined" | "cancelled" | "canceled" | "reversed" => {
                ProviderTxnStatus::Failed
            }
            _ => ProviderTxnStatus::Pending,
        })
    }

    pub async fn transaction_state(
        &self,
        transaction_key: &str,
    ) -> Result<BillerTransaction, ProviderError> {
        let url = format!("{}/biller/query/{}", self.base_url, transaction_key);
        let response = self
            .client
            .get(&url)
            .header("secretKey", self.secret_key.expose_secret())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| {
                ProviderError::transient(format!("remita query request failed: {error}"))
            })?;

        let status = response.status();
        let envelope: RemitaEnvelope = response.json().await.map_err(|error| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                format!("remita query decode failed: {error}"),
            )
        })?;

        if !status.is_success() || envelope.status != "00" {
            return Err(envelope_error(&envelope));
        }

        let data = envelope.data.ok_or_else(|| {
            ProviderError::new(
                ProviderErrorKind::InvalidResponse,
                "remita query returned no data",
            )
        })?;

        parse_transaction(data)
    }
}

fn envelope_error(envelope: &RemitaEnvelope) -> ProviderError {
    let message = format!("remita {}: {}", envelope.status, envelope.message);
    match envelope.status.as_str() {
        "96" | "99" => ProviderError::transient(message),
        "65" | "97" | "104" => ProviderError::new(ProviderErrorKind::Permanent, message),
        _ => ProviderError::new(ProviderErrorKind::InvalidResponse, message),
    }
}

fn parse_rrr_details(data: Value) -> Result<RrrDetails, ProviderError> {
    let rrr = data
        .get("rrr")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let amount_ngn = amount_from(&data, &["amountDue", "amount", "rrrAmount"]).unwrap_or(0);

    if rrr.is_empty() || amount_ngn <= 0 {
        return Err(ProviderError::new(
            ProviderErrorKind::InvalidResponse,
            "remita lookup missing rrr or amount",
        ));
    }

    let status = string_field(&data, "status").ok_or_else(|| {
        ProviderError::new(
            ProviderErrorKind::InvalidResponse,
            "remita lookup missing status",
        )
    })?;

    Ok(RrrDetails {
        biller_id: string_field(&data, "billerId").unwrap_or_default(),
        biller_name: string_field(&data, "billerName")
            .unwrap_or_else(|| String::from("Remita biller")),
        product_name: string_field(&data, "productName"),
        customer_name: string_field(&data, "payerName").or_else(|| string_field(&data, "name")),
        amount_ngn,
        fee_ngn: amount_from(&data, &["chargeFee", "fee"]).unwrap_or(0),
        rrr,
        status,
        receipt_url: data
            .get("metadata")
            .and_then(|metadata| metadata.get("receiptUrl"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        description: string_field(&data, "description"),
        raw: data,
    })
}

fn parse_transaction(data: Value) -> Result<BillerTransaction, ProviderError> {
    let status = string_field(&data, "status").ok_or_else(|| {
        ProviderError::new(
            ProviderErrorKind::InvalidResponse,
            "remita query missing status",
        )
    })?;

    Ok(BillerTransaction {
        rrr: string_field(&data, "rrr"),
        status,
        raw: data,
    })
}

fn amount_from(data: &Value, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(value) = data.get(*key) {
            return amount_from_value(Some(value));
        }
    }
    None
}

fn string_field(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn amount_from_value(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(number)) => number.as_i64().or_else(|| {
            number
                .as_f64()
                .filter(|amount| amount.fract() == 0.0)
                .map(|amount| amount as i64)
        }),
        Some(Value::String(text)) => text.trim().parse::<i64>().ok().or_else(|| {
            text.trim()
                .parse::<f64>()
                .ok()
                .filter(|amount| amount.fract() == 0.0)
                .map(|amount| amount as i64)
        }),
        _ => None,
    }
}

#[async_trait]
impl UtilityProvider for RemitaCollectionsClient {
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
        let details = self.lookup_rrr(&request.billers_code).await?;

        Ok(ValidateRefResponse {
            is_valid: details.is_payable(),
            customer_name: details.customer_name.clone(),
            settlement: Money::from_ngn_whole(details.amount_ngn).ok(),
            biller_name: Some(details.biller_name.clone()),
            product_name: details.product_name.clone(),
            reference: Some(details.rrr.clone()),
            raw: details.raw,
        })
    }

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let rrr = request.context.billers_code.clone();
        let payment_identifier = request.order_id.to_string();
        let amount = request.settlement.to_ngn_whole().map_err(|error| {
            ProviderError::permanent(format!(
                "remita collections amount is not whole naira: {error}"
            ))
        })?;

        let details = self.lookup_rrr(&rrr).await?;
        if !details.is_payable() {
            return Err(ProviderError::new(
                ProviderErrorKind::Permanent,
                format!("RRR {rrr} is not payable ({})", details.status),
            ));
        }
        if amount != details.amount_ngn {
            return Err(ProviderError::new(
                ProviderErrorKind::Permanent,
                format!(
                    "RRR {rrr} is for {} naira but the order settles {amount} naira",
                    details.amount_ngn
                ),
            ));
        }

        let status = self
            .process_rrr(&rrr, &payment_identifier, details.amount_ngn)
            .await?;

        Ok(UtilityPurchaseResponse {
            provider_reference: rrr.clone(),
            provider_request_id: rrr.clone(),
            status,
            token: Some(rrr.clone()),
            raw: json!({
                "rrr": rrr,
                "payment_identifier": payment_identifier,
                "receipt_url": details.receipt_url,
            }),
        })
    }

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError> {
        let details = self.lookup_rrr(request_id).await?;

        Ok(RequeryResponse {
            provider_request_id: request_id.to_owned(),
            status: details.transaction_status(),
            token: Some(details.rrr),
            raw: details.raw,
        })
    }

    async fn query_transaction(
        &self,
        transaction_key: &str,
    ) -> Result<RequeryResponse, ProviderError> {
        let transaction = self.transaction_state(transaction_key).await?;

        Ok(RequeryResponse {
            provider_request_id: transaction
                .rrr
                .clone()
                .unwrap_or_else(|| transaction_key.to_owned()),
            status: transaction.transaction_status(),
            token: transaction.rrr,
            raw: transaction.raw,
        })
    }

    fn verify_webhook(&self, _headers: &HeaderMap, _body: &[u8]) -> bool {
        true
    }

    fn parse_webhook_event(&self, _payload: &[u8]) -> Result<ProviderWebhookEvent, ProviderError> {
        Err(ProviderError::new(
            ProviderErrorKind::Permanent,
            "remita collections does not send webhooks",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup_fixture() -> Value {
        json!({
            "billerId": "QATEST",
            "billerName": "NSITF",
            "productName": "ADVERTISING",
            "name": "Zutility  Test",
            "paymentIdentifier": "1523991664",
            "amount": 1000,
            "fee": 0.0,
            "rrrAmount": 1000,
            "rrr": "141523991664",
            "status": "pending",
            "description": "NSITF - ADVERTISING",
            "metadata": {
                "receiptUrl": "https://receipt.remitademo.net/rrr/view/abc"
            }
        })
    }

    #[test]
    fn parses_lookup_payload() {
        let details = parse_rrr_details(lookup_fixture()).expect("details");

        assert_eq!(details.rrr, "141523991664");
        assert_eq!(details.biller_name, "NSITF");
        assert_eq!(details.amount_ngn, 1000);
        assert_eq!(details.fee_ngn, 0);
        assert_eq!(details.status, "pending");
        assert_eq!(
            details.receipt_url.as_deref(),
            Some("https://receipt.remitademo.net/rrr/view/abc")
        );
        assert!(details.is_payable());
        assert_eq!(details.transaction_status(), ProviderTxnStatus::Pending);
    }

    #[test]
    fn parses_the_documented_field_names() {
        let fixture = json!({
            "rrr": "141523991664",
            "amountDue": 7500,
            "chargeFee": 100,
            "rrrAmount": 7400,
            "payerName": "Test Payer",
            "billerName": "WAEC",
            "status": "pending"
        });
        let details = parse_rrr_details(fixture).expect("details");

        assert_eq!(details.amount_ngn, 7500);
        assert_eq!(details.fee_ngn, 100);
        assert_eq!(details.customer_name.as_deref(), Some("Test Payer"));
    }

    #[test]
    fn maps_paid_status_to_delivered() {
        let mut fixture = lookup_fixture();
        fixture["status"] = json!("paid");
        let details = parse_rrr_details(fixture).expect("details");

        assert_eq!(details.transaction_status(), ProviderTxnStatus::Delivered);
    }

    #[test]
    fn maps_failed_status_to_failed() {
        let mut fixture = lookup_fixture();
        fixture["status"] = json!("failed");
        let details = parse_rrr_details(fixture).expect("details");

        assert_eq!(details.transaction_status(), ProviderTxnStatus::Failed);
    }

    #[test]
    fn rejects_lookup_without_amount() {
        let mut fixture = lookup_fixture();
        fixture["amount"] = json!(0);

        assert!(parse_rrr_details(fixture).is_err());
    }

    #[test]
    fn paid_rrr_is_not_payable() {
        let mut fixture = lookup_fixture();
        fixture["status"] = json!("paid");
        let details = parse_rrr_details(fixture).expect("details");

        assert!(!details.is_payable());
        assert_eq!(details.transaction_status(), ProviderTxnStatus::Delivered);
    }

    #[test]
    fn rejects_lookup_without_status() {
        let mut fixture = lookup_fixture();
        if let Some(object) = fixture.as_object_mut() {
            object.remove("status");
        }

        assert!(parse_rrr_details(fixture).is_err());
    }

    #[test]
    fn rejects_fractional_amount() {
        let mut fixture = lookup_fixture();
        fixture["amount"] = json!(1000.5);

        assert!(parse_rrr_details(fixture).is_err());
    }

    fn query_fixture() -> Value {
        json!({
            "paymentIdentifier": "1524030577",
            "rrr": "251524030577",
            "amount": 300,
            "status": "pending",
            "metadata": {
                "paymentDate": "",
                "receiptUrl": "https://receipt.remitademo.net/rrr/view/abc"
            }
        })
    }

    #[test]
    fn parses_query_payload() {
        let transaction = parse_transaction(query_fixture()).expect("transaction");

        assert_eq!(transaction.rrr.as_deref(), Some("251524030577"));
        assert_eq!(transaction.status, "pending");
        assert_eq!(transaction.transaction_status(), ProviderTxnStatus::Pending);
    }

    #[test]
    fn maps_query_paid_to_delivered() {
        let mut fixture = query_fixture();
        fixture["status"] = json!("paid");
        let transaction = parse_transaction(fixture).expect("transaction");

        assert_eq!(
            transaction.transaction_status(),
            ProviderTxnStatus::Delivered
        );
    }

    #[test]
    fn maps_query_failed_to_failed() {
        let mut fixture = query_fixture();
        fixture["status"] = json!("failed");
        let transaction = parse_transaction(fixture).expect("transaction");

        assert_eq!(transaction.transaction_status(), ProviderTxnStatus::Failed);
    }

    #[test]
    fn rejects_query_without_status() {
        let mut fixture = query_fixture();
        if let Some(object) = fixture.as_object_mut() {
            object.remove("status");
        }

        assert!(parse_transaction(fixture).is_err());
    }
}
