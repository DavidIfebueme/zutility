use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

pub const CATEGORY_ORDER: [&str; 6] = [
    "airtime",
    "data",
    "television",
    "electricity",
    "education",
    "school",
];

pub const CATALOGUE_NAMESPACE: Uuid = Uuid::from_u128(0x8f1ca8f3_e61d_53d5_b939_6d574b1396de);

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct CatalogueVariation {
    pub variation_code: String,
    pub name: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct CatalogueProduct {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub descriptor: String,
    pub icon_key: String,
    pub service_ref_label: String,
    pub service_ref_placeholder: String,
    pub service_ref_pattern: Option<String>,
    pub quick_amounts: Option<Value>,
    pub amount_kind: String,
    pub amount_min: Option<String>,
    pub amount_max: Option<String>,
    pub amount_increment: Option<String>,
    pub currency: String,
    pub fulfilment_kind: String,
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    pub required_fields: Value,
    pub field_labels: Value,
    pub variations: Vec<CatalogueVariation>,
    #[serde(skip)]
    #[schema(ignore)]
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct CatalogueCategory {
    pub category: String,
    pub products: Vec<CatalogueProduct>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct CatalogueCountry {
    pub iso2: String,
    pub name: String,
    pub currency: String,
    pub categories: Vec<CatalogueCategory>,
}

pub fn product_id(provider: &str, provider_product_id: &str) -> Uuid {
    let name = format!("{provider}:{provider_product_id}");
    Uuid::new_v5(&CATALOGUE_NAMESPACE, name.as_bytes())
}

pub fn group_by_category(products: Vec<CatalogueProduct>) -> Vec<CatalogueCategory> {
    let mut groups: Vec<(String, Vec<CatalogueProduct>)> = Vec::new();
    for product in products {
        match groups
            .iter_mut()
            .find(|(category, _)| *category == product.category)
        {
            Some((_, items)) => items.push(product),
            None => groups.push((product.category.clone(), vec![product])),
        }
    }
    groups.sort_by_key(|(category, _)| {
        CATEGORY_ORDER
            .iter()
            .position(|known| *known == category.as_str())
            .unwrap_or(usize::MAX)
    });
    groups
        .into_iter()
        .map(|(category, products)| CatalogueCategory { category, products })
        .collect()
}
