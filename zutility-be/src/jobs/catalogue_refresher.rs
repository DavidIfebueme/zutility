use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::PgPool;

use crate::{
    db,
    integrations::dtone::{DtoneClient, PROVIDER_NAME},
};

pub const DTONE_CATALOGUE_SERVICE_ID: u32 = 3;

pub struct CatalogueRefresher {
    client: Arc<DtoneClient>,
}

impl CatalogueRefresher {
    pub fn new(client: Arc<DtoneClient>) -> Self {
        Self { client }
    }

    pub async fn refresh_once(&self, pool: &PgPool) -> Result<usize> {
        let products = db::list_enabled_products_by_provider(pool, PROVIDER_NAME).await?;

        let mut countries: Vec<(String, String)> = Vec::new();
        for product in &products {
            if !countries
                .iter()
                .any(|(iso2, _)| iso2 == &product.country_iso2)
            {
                countries.push((product.country_iso2.clone(), product.country_iso3.clone()));
            }
        }

        let mut updated = 0usize;
        for (country_iso2, country_iso3) in countries {
            let fetched = match self
                .client
                .products(&country_iso3, DTONE_CATALOGUE_SERVICE_ID)
                .await
            {
                Ok(products) => products,
                Err(error) => {
                    tracing::warn!(
                        country_iso2,
                        error = %error,
                        "catalogue refresh failed for a country, keeping the stored rows"
                    );
                    continue;
                }
            };

            let by_id: HashMap<String, Value> = fetched
                .into_iter()
                .map(|product| (product.id, product.raw))
                .collect();

            for product in products
                .iter()
                .filter(|product| product.country_iso2 == country_iso2)
            {
                let Some(raw) = by_id.get(&product.provider_product_id) else {
                    continue;
                };

                let fetched_at = Utc::now();
                match rates_from_value(raw) {
                    Some((wholesale, retail)) => {
                        db::update_product_rates(pool, product.id, wholesale, retail, fetched_at)
                            .await?;
                    }
                    None => {
                        db::stamp_product_synced_at(pool, product.id, fetched_at).await?;
                    }
                }
                updated += 1;
            }
        }

        Ok(updated)
    }
}

fn rates_from_value(raw: &Value) -> Option<(Decimal, Option<Decimal>)> {
    let rates = raw.get("rates")?;
    let wholesale = decimal_at(rates, "wholesale")?;
    if wholesale <= Decimal::ZERO {
        return None;
    }
    let retail = decimal_at(rates, "retail").filter(|retail| *retail > Decimal::ZERO);
    Some((wholesale, retail))
}

fn decimal_at(value: &Value, key: &str) -> Option<Decimal> {
    match value.get(key)? {
        Value::Number(number) => number.to_string().parse::<Decimal>().ok(),
        Value::String(text) => text.trim().parse::<Decimal>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_wholesale_and_retail_rates() {
        let raw = json!({"rates": {"wholesale": "1.25", "retail": 1.40}});
        assert_eq!(
            rates_from_value(&raw),
            Some((Decimal::new(125, 2), Some(Decimal::new(140, 2))))
        );
    }

    #[test]
    fn a_missing_wholesale_rate_reads_as_none() {
        assert_eq!(rates_from_value(&json!({"rates": {"retail": 1.40}})), None);
        assert_eq!(rates_from_value(&json!({})), None);
    }
}
