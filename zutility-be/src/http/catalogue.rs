use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{db, domain::catalogue::CatalogueCountry};

use super::{error::ApiError, handlers::HttpState};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CatalogueQuery {
    pub country: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogueResponse {
    pub countries: Vec<CatalogueCountry>,
}

#[utoipa::path(
    get,
    path = "/api/v1/catalogue",
    params(
        ("country" = Option<String>, Query, description = "ISO 3166-1 alpha-2 country code")
    ),
    responses(
        (status = 200, description = "Enabled country catalogue", body = CatalogueResponse),
        (status = 500, description = "Internal error", body = super::error::ErrorEnvelope)
    )
)]
pub async fn get_catalogue(
    State(state): State<HttpState>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, ApiError> {
    let country = query
        .country
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let countries = db::list_catalogue(&state.pool, country.as_deref())
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "catalogue query failed");
            ApiError::internal("something went wrong on our side. Try again.")
        })?;

    Ok(Json(CatalogueResponse { countries }))
}
