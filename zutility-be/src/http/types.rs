use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::gift::GiftStatus;
use crate::domain::money::{CurrencyCode, Money};
use crate::domain::order::OrderStatus;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MoneyDto {
    pub currency: String,
    pub amount: String,
}

impl MoneyDto {
    pub fn from_money(money: Money) -> Self {
        Self {
            currency: money.currency().iso().to_string(),
            amount: money.round_to_currency().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DisplayMoney {
    pub currency: String,
    pub amount: Option<String>,
    pub rate_available: bool,
}

impl DisplayMoney {
    pub fn target_currency(override_iso: Option<&str>, preferred: CurrencyCode) -> CurrencyCode {
        override_iso
            .and_then(CurrencyCode::from_iso)
            .unwrap_or(preferred)
    }

    pub fn new(
        currency: CurrencyCode,
        amount: Option<Money>,
        rate: Option<(Decimal, Decimal)>,
    ) -> Self {
        let converted = match (currency, amount) {
            (CurrencyCode::Ngn, Some(money)) => Some(money),
            (_, Some(money)) => {
                rate.and_then(|(usd_ngn, per_usd)| money.display_in(currency, usd_ngn, per_usd))
            }
            (_, None) => None,
        };

        Self {
            currency: currency.iso().to_string(),
            amount: converted.map(|money| money.round_to_currency().to_string()),
            rate_available: converted.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FxQuoteResponse {
    pub quote: String,
    pub per_usd: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SettlementRequest {
    pub currency: String,
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub utility_type: String,
    pub utility_slug: String,
    pub service_ref: String,
    #[serde(default)]
    pub amount_ngn: Option<i64>,
    pub zec_address_type: String,
    pub variation_code: Option<String>,
    pub meter_type: Option<String>,
    pub country_iso2: Option<String>,
    pub product_id: Option<Uuid>,
    pub settlement: Option<SettlementRequest>,
    #[serde(default)]
    pub inputs: std::collections::BTreeMap<String, String>,
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
    pub address_type: String,
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
    pub deposit_address: String,
    pub qr_data: String,
    pub address_type: String,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub delivery_token: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub settlement: Option<MoneyDto>,
    pub fulfilment_kind: String,
    pub display: DisplayMoney,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CancelOrderResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrderAccessTokenResponse {
    pub order_access_token: String,
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
    pub fx: Vec<FxQuoteResponse>,
    pub display: DisplayMoney,
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
    pub amount_ngn: Option<i64>,
    pub biller_name: Option<String>,
    pub product_name: Option<String>,
    pub reference: Option<String>,
    pub display: DisplayMoney,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderTokenQuery {
    pub token: String,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtilityValidateQuery {
    #[serde(rename = "ref")]
    pub reference: String,
    pub meter_type: Option<String>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrencyQuery {
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: Option<String>,
    pub password: String,
    pub preferred_currency: Option<String>,
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
    pub preferred_currency: String,
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
    pub display: DisplayMoney,
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub preferred_currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeleteAccountRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub order_id: Option<Uuid>,
    pub r#type: String,
    pub title: String,
    pub body: String,
    pub detail: Value,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateGiftRequest {
    pub utility_type: String,
    pub utility_slug: String,
    pub amount_ngn: i64,
    pub zec_address_type: String,
    pub variation_code: Option<String>,
    pub message: Option<String>,
    pub fallback_service_ref: Option<String>,
    pub claim_window_hours: i64,
    pub hide_sender_name: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateGiftResponse {
    pub gift_id: Uuid,
    pub gift_code: String,
    pub gift_url: String,
    pub claim_window_hours: i64,
    pub order: CreateOrderResponse,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ClaimGiftRequest {
    pub service_ref: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GiftView {
    pub gift_id: Uuid,
    pub zec_amount: Option<String>,
    pub utility_type: String,
    pub utility_slug: String,
    pub amount_ngn: i64,
    pub message: Option<String>,
    pub sender_name: String,
    pub status: GiftStatus,
    pub claimed: bool,
    pub claim_expires_at: Option<DateTime<Utc>>,
    pub order_status: Option<OrderStatus>,
    pub confirmations: u16,
    pub required_confirmations: u16,
    pub delivery_token: Option<String>,
    pub display: DisplayMoney,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GiftSummary {
    pub gift_id: Uuid,
    pub order_id: Option<Uuid>,
    pub utility_type: String,
    pub utility_slug: String,
    pub amount_ngn: i64,
    pub status: GiftStatus,
    pub order_status: Option<OrderStatus>,
    pub claimed_service_ref: Option<String>,
    pub claim_window_hours: i64,
    pub claim_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub display: DisplayMoney,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RotateGiftCodeResponse {
    pub gift_code: String,
    pub gift_url: String,
}
