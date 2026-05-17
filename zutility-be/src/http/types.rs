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
    pub variation_code: Option<String>,
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
    pub utility_slug: String,
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
    pub usd_ngn: String,
    pub usd_kes: String,
    pub usd_ghs: String,
    pub usd_zar: String,
    pub usd_egp: String,
    pub updated_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UtilityItem {
    pub slug: String,
    pub utility_type: String,
    pub name: String,
    pub field_config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UtilityVariationItem {
    pub variation_code: String,
    pub name: String,
    pub amount: Option<i64>,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: Option<String>,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthUserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrderHistoryItem {
    pub order_id: Uuid,
    pub utility_slug: String,
    pub utility_type: String,
    pub amount_ngn: i64,
    pub zec_amount: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
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
    pub variation_code: Option<String>,
    pub provider: Option<String>,
    pub customer_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WaitlistJoinRequest {
    pub email: String,
    pub display_name: Option<String>,
    pub ref_code: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub utm_content: Option<String>,
    pub utm_term: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WaitlistVerifyRequest {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct WaitlistResendRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WaitlistJoinResponse {
    pub referral_code: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WaitlistVerifyResponse {
    pub email: String,
    pub position: i64,
    pub referral_code: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WaitlistStatsResponse {
    pub total: i64,
    pub verified: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SupportRequest {
    pub email: String,
    pub name: String,
    pub subject: String,
    pub message: String,
}
