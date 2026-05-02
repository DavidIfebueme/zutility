use std::sync::Arc;

use async_trait::async_trait;

use crate::integrations::{
    remita::RemitaClient,
    utility_provider::{
        ProviderError, ProviderKind, UtilityProvider, UtilityProviderRouter,
        UtilityPurchaseRequest, UtilityPurchaseResponse, UtilityVariation, ValidateRefRequest,
        ValidateRefResponse, RequeryResponse, ProviderWebhookEvent,
    },
    vtpass::VtpassClient,
};

#[derive(Clone)]
pub struct ProviderDispatcher {
    vtpass: Arc<VtpassClient>,
    remita: Option<Arc<RemitaClient>>,
}

impl ProviderDispatcher {
    pub fn new(vtpass: VtpassClient, remita: Option<RemitaClient>) -> Self {
        Self {
            vtpass: Arc::new(vtpass),
            remita: remita.map(Arc::new),
        }
    }

    pub fn provider_for(&self, utility_type: &str) -> Result<Arc<dyn UtilityProvider>, ProviderError> {
        match utility_type {
            "school_fees" => {
                match self.remita.clone() {
                    Some(r) => Ok(r as Arc<dyn UtilityProvider>),
                    None => Err(ProviderError::new(
                        crate::integrations::utility_provider::ProviderErrorKind::Outage,
                        "Remita is not configured. Set REMITA_* environment variables to enable school fees payments.",
                    )),
                }
            }
            _ => Ok(self.vtpass.clone() as Arc<dyn UtilityProvider>),
        }
    }

    pub async fn service_variations(
        &self,
        utility_type: &str,
        service_id: &str,
    ) -> Result<Vec<UtilityVariation>, ProviderError> {
        self.provider_for(utility_type)?
            .service_variations(service_id)
            .await
    }

    pub async fn validate_reference(
        &self,
        utility_type: &str,
        request: &ValidateRefRequest,
    ) -> Result<ValidateRefResponse, ProviderError> {
        self.provider_for(utility_type)?
            .validate_reference(request)
            .await
    }

    pub async fn pay(
        &self,
        utility_type: &str,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let provider = self.provider_for(utility_type)?;
        let result = provider.pay(request).await;

        if utility_type == "electricity" {
            if let Err(ref error) = result {
                if matches!(
                    error.kind,
                    crate::integrations::utility_provider::ProviderErrorKind::Outage
                        | crate::integrations::utility_provider::ProviderErrorKind::Transient
                ) {
                    if let Some(ref remita) = self.remita {
                        return remita.pay(request).await;
                    }
                }
            }
        }

        result
    }

    pub async fn requery(
        &self,
        utility_type: &str,
        request_id: &str,
    ) -> Result<RequeryResponse, ProviderError> {
        self.provider_for(utility_type)?.requery(request_id).await
    }

    pub fn verify_webhook_signature(
        &self,
        utility_type: &str,
        payload: &[u8],
        signature: &str,
    ) -> bool {
        match self.provider_for(utility_type) {
            Ok(p) => p.verify_webhook_signature(payload, signature),
            Err(_) => false,
        }
    }

    pub fn provider_kind_for(&self, utility_type: &str) -> ProviderKind {
        match utility_type {
            "school_fees" if self.remita.is_some() => ProviderKind::Remita,
            _ => ProviderKind::Vtpass,
        }
    }
}
