use std::{fmt::Display, sync::Arc};

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Vtpass,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Transient,
    Outage,
    Permanent,
    Unauthorized,
    InvalidResponse,
}

#[derive(Debug, Clone)]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
}

impl ProviderError {
    pub fn new(kind: ProviderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn transient(message: impl Into<String>) -> Self {
        Self::new(ProviderErrorKind::Transient, message)
    }

    pub fn outage(message: impl Into<String>) -> Self {
        Self::new(ProviderErrorKind::Outage, message)
    }

    pub fn permanent(message: impl Into<String>) -> Self {
        Self::new(ProviderErrorKind::Permanent, message)
    }
}

impl std::error::Error for ProviderError {}

impl Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityVariation {
    pub variation_code: String,
    pub name: String,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ValidateRefRequest {
    pub service_id: String,
    pub billers_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRefResponse {
    pub is_valid: bool,
    pub customer_name: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct UtilityPurchaseRequest {
    pub order_id: Uuid,
    pub request_id: String,
    pub service_id: String,
    pub billers_code: String,
    pub variation_code: Option<String>,
    pub amount_ngn: i64,
    pub phone: Option<String>,
    pub metadata: serde_json::Value,
    pub zec_amount: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderTxnStatus {
    Pending,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityPurchaseResponse {
    pub provider_reference: String,
    pub provider_request_id: String,
    pub status: ProviderTxnStatus,
    pub token: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequeryResponse {
    pub provider_request_id: String,
    pub status: ProviderTxnStatus,
    pub token: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWebhookEvent {
    pub provider_request_id: String,
    pub status: ProviderTxnStatus,
    pub token: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutageRunbookAction {
    PauseAffectedUtilityOrders,
    KeepStatusAndRequeryWorkersActive,
    ResumeWithAdminToggle,
}

pub fn outage_runbook_actions() -> Vec<OutageRunbookAction> {
    vec![
        OutageRunbookAction::PauseAffectedUtilityOrders,
        OutageRunbookAction::KeepStatusAndRequeryWorkersActive,
        OutageRunbookAction::ResumeWithAdminToggle,
    ]
}

#[async_trait]
pub trait UtilityProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    async fn service_variations(
        &self,
        service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError>;

    async fn validate_reference(
        &self,
        request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError>;

    async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError>;

    async fn requery(&self, request_id: &str) -> Result<RequeryResponse, ProviderError>;

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool;

    fn parse_webhook_event(&self, payload: &[u8]) -> Result<ProviderWebhookEvent, ProviderError>;
}

#[derive(Clone)]
pub struct UtilityProviderRouter {
    primary: Arc<dyn UtilityProvider>,
    secondary: Option<Arc<dyn UtilityProvider>>,
    enable_failover: bool,
}

impl UtilityProviderRouter {
    pub fn new(primary: Arc<dyn UtilityProvider>) -> Self {
        Self {
            primary,
            secondary: None,
            enable_failover: false,
        }
    }

    pub fn with_secondary(
        mut self,
        secondary: Arc<dyn UtilityProvider>,
        enable_failover: bool,
    ) -> Self {
        self.secondary = Some(secondary);
        self.enable_failover = enable_failover;
        self
    }

    pub fn failover_enabled(&self) -> bool {
        self.enable_failover && self.secondary.is_some()
    }

    pub async fn pay(
        &self,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        match self.primary.pay(request).await {
            Ok(response) => Ok(response),
            Err(error)
                if self.failover_enabled()
                    && matches!(
                        error.kind,
                        ProviderErrorKind::Outage | ProviderErrorKind::Transient
                    ) =>
            {
                match &self.secondary {
                    Some(secondary) => secondary.pay(request).await,
                    None => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    use super::{
        ProviderError, ProviderErrorKind, ProviderKind, RequeryResponse, UtilityProvider,
        UtilityProviderRouter, UtilityPurchaseRequest, UtilityPurchaseResponse, UtilityVariation,
        ValidateRefRequest, ValidateRefResponse,
    };

    #[derive(Clone)]
    struct MockProvider {
        kind: ProviderKind,
        should_fail: bool,
    }

    #[async_trait]
    impl UtilityProvider for MockProvider {
        fn kind(&self) -> ProviderKind {
            self.kind
        }

        async fn service_variations(
            &self,
            _service_id: &str,
        ) -> Result<Vec<UtilityVariation>, ProviderError> {
            Ok(Vec::new())
        }

        async fn validate_reference(
            &self,
            _request: &ValidateRefRequest,
        ) -> Result<ValidateRefResponse, ProviderError> {
            Ok(ValidateRefResponse {
                is_valid: true,
                customer_name: Some(String::from("ok")),
                raw: serde_json::json!({}),
            })
        }

        async fn pay(
            &self,
            request: &UtilityPurchaseRequest,
        ) -> Result<UtilityPurchaseResponse, ProviderError> {
            if self.should_fail {
                return Err(ProviderError::new(
                    ProviderErrorKind::Outage,
                    "provider unavailable",
                ));
            }
            Ok(UtilityPurchaseResponse {
                provider_reference: request.order_id.to_string(),
                provider_request_id: request.request_id.clone(),
                status: super::ProviderTxnStatus::Delivered,
                token: None,
                raw: serde_json::json!({}),
            })
        }

        async fn requery(&self, _request_id: &str) -> Result<RequeryResponse, ProviderError> {
            Ok(RequeryResponse {
                provider_request_id: String::from("req"),
                status: super::ProviderTxnStatus::Pending,
                token: None,
                raw: serde_json::json!({}),
            })
        }

        fn verify_webhook_signature(&self, _payload: &[u8], _signature: &str) -> bool {
            true
        }

        fn parse_webhook_event(
            &self,
            _payload: &[u8],
        ) -> Result<super::ProviderWebhookEvent, ProviderError> {
            Ok(super::ProviderWebhookEvent {
                provider_request_id: String::from("req"),
                status: super::ProviderTxnStatus::Pending,
                token: None,
                raw: serde_json::json!({}),
            })
        }
    }

    #[tokio::test]
    async fn routes_to_secondary_on_primary_outage_when_enabled() {
        let primary = Arc::new(MockProvider {
            kind: ProviderKind::Vtpass,
            should_fail: true,
        });
        let secondary = Arc::new(MockProvider {
            kind: ProviderKind::Secondary,
            should_fail: false,
        });

        let router = UtilityProviderRouter::new(primary).with_secondary(secondary, true);
        let request = UtilityPurchaseRequest {
            order_id: Uuid::new_v4(),
            request_id: String::from("req-id"),
            service_id: String::from("mtn"),
            billers_code: String::from("080"),
            variation_code: None,
            amount_ngn: 1000,
            phone: None,
            metadata: serde_json::json!({}),
            zec_amount: Decimal::new(1, 0),
        };

        let result = router.pay(&request).await;
        assert!(result.is_ok());
    }
}
