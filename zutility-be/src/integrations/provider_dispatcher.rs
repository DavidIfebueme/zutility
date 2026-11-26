use std::sync::Arc;

use axum::http::HeaderMap;

use crate::integrations::{
    dtone::DtoneClient,
    inlomax::InlomaxClient,
    remita::RemitaClient,
    remita_collections::RemitaCollectionsClient,
    utility_provider::{
        ProviderError, ProviderErrorKind, ProviderKind, RequeryResponse, UtilityProvider,
        UtilityPurchaseRequest, UtilityPurchaseResponse, UtilityVariation, ValidateRefRequest,
        ValidateRefResponse,
    },
    vtpass::VtpassClient,
};
use crate::observability::MetricsRegistry;

const INLOMAX_UTILITY_TYPES: &[&str] = &[
    "airtime",
    "data",
    "dstv",
    "gotv",
    "startimes",
    "electricity",
    "waec",
    "jamb",
];

#[derive(Clone)]
pub struct ProviderDispatcher {
    inlomax: Option<Arc<InlomaxClient>>,
    vtpass: Arc<VtpassClient>,
    remita: Option<Arc<RemitaClient>>,
    remita_collections: Option<Arc<RemitaCollectionsClient>>,
    dtone: Option<Arc<DtoneClient>>,
    metrics: Option<Arc<MetricsRegistry>>,
}

impl ProviderDispatcher {
    pub fn new(
        vtpass: VtpassClient,
        remita: Option<RemitaClient>,
        remita_collections: Option<RemitaCollectionsClient>,
        inlomax: Option<InlomaxClient>,
        dtone: Option<DtoneClient>,
    ) -> Self {
        Self {
            inlomax: inlomax.map(Arc::new),
            vtpass: Arc::new(vtpass),
            remita: remita.map(Arc::new),
            remita_collections: remita_collections.map(Arc::new),
            dtone: dtone.map(Arc::new),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<MetricsRegistry>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn dtone(&self) -> Option<Arc<DtoneClient>> {
        self.dtone.clone()
    }

    pub fn provider_name_is_known(provider: &str) -> bool {
        matches!(
            provider,
            "inlomax" | "vtpass" | "remita" | "remita_collections" | "dtone"
        )
    }

    pub fn is_configured(&self, provider: &str) -> bool {
        match provider {
            "inlomax" => self.inlomax.is_some(),
            "vtpass" => true,
            "remita" => self.remita.is_some(),
            "remita_collections" => self.remita_collections.is_some(),
            "dtone" => self.dtone.is_some(),
            _ => false,
        }
    }

    pub fn provider_for_name(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn UtilityProvider>, ProviderError> {
        match provider {
            "inlomax" => match self.inlomax.clone() {
                Some(client) => Ok(client as Arc<dyn UtilityProvider>),
                None => Err(ProviderError::outage("inlomax is not configured")),
            },
            "vtpass" => Ok(self.vtpass.clone() as Arc<dyn UtilityProvider>),
            "remita" => match self.remita.clone() {
                Some(client) => Ok(client as Arc<dyn UtilityProvider>),
                None => Err(ProviderError::outage("remita is not configured")),
            },
            "remita_collections" => match self.remita_collections.clone() {
                Some(client) => Ok(client as Arc<dyn UtilityProvider>),
                None => Err(ProviderError::outage(
                    "remita collections is not configured",
                )),
            },
            "dtone" => match self.dtone.clone() {
                Some(client) => Ok(client as Arc<dyn UtilityProvider>),
                None => Err(ProviderError::outage("dt one is not configured")),
            },
            other => Err(ProviderError::permanent(format!(
                "unknown provider {other} for this product"
            ))),
        }
    }

    pub async fn pay_for_product(
        &self,
        provider: &str,
        failover: Option<&str>,
        request: &UtilityPurchaseRequest,
    ) -> Result<UtilityPurchaseResponse, ProviderError> {
        let primary = self.provider_for_name(provider)?;
        let result = primary.pay(request).await;

        let Err(error) = result else {
            return result;
        };

        self.record_provider_error(primary.kind());

        if matches!(error.kind, ProviderErrorKind::Outage)
            && let Some(failover) = failover
            && self.is_configured(failover)
            && failover != provider
        {
            tracing::warn!(
                provider,
                failover,
                error = %error,
                "product provider outage, falling back to the recorded failover"
            );
            let secondary = self.provider_for_name(failover)?;
            let retried = secondary.pay(request).await;
            if retried.is_err() {
                self.record_provider_error(secondary.kind());
            }
            return retried;
        }

        Err(error)
    }

    pub async fn requery_for_product(
        &self,
        provider: &str,
        request_id: &str,
    ) -> Result<RequeryResponse, ProviderError> {
        let provider = self.provider_for_name(provider)?;
        let result = provider.requery(request_id).await;
        if result.is_err() {
            self.record_provider_error(provider.kind());
        }
        result
    }

    pub fn warn_unconfigured_providers(&self, rows: &[(String, Option<String>)]) {
        for (provider, failover) in rows {
            if !Self::provider_name_is_known(provider) || !self.is_configured(provider) {
                tracing::warn!(
                    provider,
                    "enabled product resolves to a provider adapter that is not configured"
                );
            }
            if let Some(failover) = failover
                && (!Self::provider_name_is_known(failover) || !self.is_configured(failover))
            {
                tracing::warn!(
                    provider,
                    failover,
                    "enabled product names a failover adapter that is not configured"
                );
            }
        }
    }

    fn record_provider_error(&self, kind: ProviderKind) {
        if let Some(metrics) = self.metrics.as_ref() {
            metrics.record_provider_error(kind.as_label());
        }
    }

    pub fn provider_for(
        &self,
        utility_type: &str,
    ) -> Result<Arc<dyn UtilityProvider>, ProviderError> {
        match utility_type {
            "school_fees" => match self.remita_collections.clone() {
                Some(r) => Ok(r as Arc<dyn UtilityProvider>),
                None => Err(ProviderError::new(
                    ProviderErrorKind::Outage,
                    "Remita Collections is not configured. Set REMITA_COLLECTIONS_BASE_URL and REMITA_COLLECTIONS_SECRET_KEY to enable school fees payments.",
                )),
            },
            ut if INLOMAX_UTILITY_TYPES.contains(&ut) => match self.inlomax.clone() {
                Some(i) => Ok(i as Arc<dyn UtilityProvider>),
                None => Ok(self.vtpass.clone() as Arc<dyn UtilityProvider>),
            },
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
            self.record_provider_error(provider.kind());
            if matches!(error.kind, ProviderErrorKind::Outage) {
                if INLOMAX_UTILITY_TYPES.contains(&utility_type) && self.inlomax.is_some() {
                    tracing::warn!(error = %error, "inlomax outage, falling back to vtpass");
                    return self.vtpass.pay(request).await;
                }
                if utility_type == "electricity"
                    && let Some(ref remita) = self.remita
                {
                    tracing::warn!(error = %error, "primary electricity outage, falling back to remita");
                    return remita.pay(request).await;
                }
            }
            if matches!(error.kind, ProviderErrorKind::Transient) {
                tracing::warn!(error = %error, "provider transient error, NOT falling back — will requery next cycle");
            }
        }

        result
    }

    pub async fn requery(
        &self,
        utility_type: &str,
        request_id: &str,
    ) -> Result<RequeryResponse, ProviderError> {
        let provider = self.provider_for(utility_type)?;
        let result = provider.requery(request_id).await;
        if result.is_err() {
            self.record_provider_error(provider.kind());
        }
        result
    }

    pub fn verify_webhook(&self, utility_type: &str, headers: &HeaderMap, body: &[u8]) -> bool {
        match self.provider_for(utility_type) {
            Ok(p) => p.verify_webhook(headers, body),
            Err(_) => false,
        }
    }

    pub fn provider_kind_for(&self, utility_type: &str) -> ProviderKind {
        match utility_type {
            "school_fees" if self.remita_collections.is_some() => ProviderKind::Remita,
            ut if INLOMAX_UTILITY_TYPES.contains(&ut) && self.inlomax.is_some() => {
                ProviderKind::Inlomax
            }
            _ => ProviderKind::Vtpass,
        }
    }
}
