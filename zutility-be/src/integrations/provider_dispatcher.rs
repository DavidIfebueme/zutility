use std::sync::Arc;

use crate::integrations::{
    inlomax::InlomaxClient,
    remita::RemitaClient,
    utility_provider::{
        ProviderError, ProviderKind, UtilityProvider,
        UtilityPurchaseRequest, UtilityPurchaseResponse, UtilityVariation, ValidateRefRequest,
        ValidateRefResponse, RequeryResponse,
    },
    vtpass::VtpassClient,
};

const INLOMAX_UTILITY_TYPES: &[&str] = &[
    "airtime", "data", "dstv", "gotv", "startimes", "electricity", "waec", "jamb",
];

#[derive(Clone)]
pub struct ProviderDispatcher {
    inlomax: Option<Arc<InlomaxClient>>,
    vtpass: Arc<VtpassClient>,
    remita: Option<Arc<RemitaClient>>,
}

impl ProviderDispatcher {
    pub fn new(vtpass: VtpassClient, remita: Option<RemitaClient>, inlomax: Option<InlomaxClient>) -> Self {
        Self {
            inlomax: inlomax.map(Arc::new),
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
            ut if INLOMAX_UTILITY_TYPES.contains(&ut) => {
                match self.inlomax.clone() {
                    Some(i) => Ok(i as Arc<dyn UtilityProvider>),
                    None => Ok(self.vtpass.clone() as Arc<dyn UtilityProvider>),
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

        if let Err(ref error) = result {
            if matches!(
                error.kind,
                crate::integrations::utility_provider::ProviderErrorKind::Outage
                    | crate::integrations::utility_provider::ProviderErrorKind::Transient
            ) {
                if INLOMAX_UTILITY_TYPES.contains(&utility_type) && self.inlomax.is_some() {
                    return self.vtpass.pay(request).await;
                }
                if utility_type == "electricity" {
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
            ut if INLOMAX_UTILITY_TYPES.contains(&ut) && self.inlomax.is_some() => ProviderKind::Inlomax,
            _ => ProviderKind::Vtpass,
        }
    }
}
