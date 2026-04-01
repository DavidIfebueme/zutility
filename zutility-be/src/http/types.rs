use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::order::OrderStatus;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub zec_address_type: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateOrderResponse {
    pub order_id: Uuid,
    pub order_access_token: String,
    pub deposit_address: String,
    pub zec_amount: String,
    pub expires_at: DateTime<Utc>,
    pub qr_data: String,
    pub required_confirmations: u16,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrderStatusResponse {
    pub order_id: Uuid,
    pub status: OrderStatus,
    pub confirmations: u16,
    pub required_confirmations: u16,
    pub total_received: Option<String>,
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub zec_amount: String,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub delivery_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CancelOrderResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RateResponse {
    pub zec_ngn: String,
    pub zec_usd: String,
    pub updated_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UtilityItem {
    pub slug: String,
    pub utility_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UtilityValidateResponse {
    pub valid: bool,
    pub customer_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderTokenQuery {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtilityValidateQuery {
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct OrderRecord {
    pub order_id: Uuid,
    pub access_token_hash: String,
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    pub amount_ngn: i64,
    pub zec_amount: Decimal,
    pub deposit_address: String,
    pub status: OrderStatus,
    pub confirmations: u16,
    pub required_confirmations: u16,
    pub total_received: Option<Decimal>,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub delivery_token: Option<String>,
}
