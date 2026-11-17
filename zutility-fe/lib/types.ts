import type { DisplayMoney } from '@/lib/money'

export type OrderStatus =
  | 'awaiting_payment' | 'payment_detected' | 'payment_confirmed' | 'awaiting_claim'
  | 'utility_dispatching' | 'completed' | 'expired' | 'failed' | 'flagged_for_review' | 'cancelled'
  | 'reversed'

export type SettlementCurrency = { currency: string; amount: string }

export interface CreateOrderRequest {
  utility_type: string
  utility_slug: string
  service_ref: string
  amount_ngn?: number
  zec_address_type: 'shielded' | 'transparent'
  variation_code?: string
  meter_type?: 'prepaid' | 'postpaid'
  country_iso2?: string
  product_id?: string
  settlement?: SettlementCurrency
  inputs?: Record<string, string>
}

export interface CreateOrderResponse {
  order_id: string
  order_access_token: string
  deposit_address: string
  zec_amount: string
  expires_at: string
  qr_data: string
  required_confirmations: number
  address_type: 'shielded' | 'transparent'
  utility_slug: string
}

export interface OrderStatusResponse {
  order_id: string
  status: OrderStatus
  confirmations: number
  required_confirmations: number
  total_received: string | null
  utility_type: string
  utility_slug: string
  service_ref: string
  amount_ngn: number
  zec_amount: string
  deposit_address: string
  qr_data: string
  address_type: 'shielded' | 'transparent'
  expires_at: string
  completed_at: string | null
  delivery_token: string | null
  failure_code: string | null
  failure_message: string | null
  settlement: SettlementCurrency | null
  fulfilment_kind: string
  display: DisplayMoney
}

export interface OrderHistoryItem {
  order_id: string
  utility_slug: string
  utility_type: string
  amount_ngn: number
  zec_amount: string
  status: OrderStatus
  created_at: string
  completed_at: string | null
  display: DisplayMoney
}

export interface FxQuote {
  quote: string
  per_usd: string
  fetched_at: string
}

export interface RateResponse {
  zec_ngn: string
  zec_usd: string
  usd_ngn: string
  usd_kes: string
  usd_ghs: string
  usd_zar: string
  usd_egp: string
  updated_at: string
  valid_until: string
  fx: FxQuote[]
  display: DisplayMoney
}

export interface UtilityVariationResponse {
  variation_code: string
  name: string
  amount: number | null
}

export interface UtilityValidateResponse {
  valid: boolean
  customer_name: string | null
  amount_ngn: number | null
  biller_name: string | null
  product_name: string | null
  reference: string | null
  display: DisplayMoney
}

export type OrderStreamEvent =
  | { event: 'payment_detected'; confirmations: number; required: number }
  | { event: 'confirmation'; confirmations: number; required: number }
  | { event: 'payment_confirmed'; confirmations: number }
  | { event: 'awaiting_claim' }
  | { event: 'dispatching' }
  | { event: 'completed'; delivery_token: string | null; reference: string }
  | { event: 'expired' }
  | { event: 'failed'; reason: string }
  | { event: 'flagged'; reason: string }
  | { event: 'cancelled' }
  | { event: 'reversed' }

export type GiftStatus = 'open' | 'claimed' | 'cancelled' | 'expired'

export interface CreateGiftRequest {
  utility_type: string
  utility_slug: string
  amount_ngn: number
  zec_address_type: 'shielded' | 'transparent'
  variation_code?: string
  message?: string
  fallback_service_ref?: string
  claim_window_hours: number
  hide_sender_name?: boolean
}

export interface CreateGiftResponse {
  gift_id: string
  gift_code: string
  gift_url: string
  claim_window_hours: number
  order: CreateOrderResponse
}

export interface GiftView {
  gift_id: string
  zec_amount: string | null
  utility_type: string
  utility_slug: string
  amount_ngn: number
  message: string | null
  sender_name: string
  status: GiftStatus
  claimed: boolean
  claim_expires_at: string | null
  order_status: OrderStatus | null
  confirmations: number
  required_confirmations: number
  delivery_token: string | null
  display: DisplayMoney
}

export interface GiftSummary {
  gift_id: string
  order_id: string | null
  utility_type: string
  utility_slug: string
  amount_ngn: number
  status: GiftStatus
  order_status: OrderStatus | null
  claimed_service_ref: string | null
  claim_window_hours: number
  claim_expires_at: string | null
  created_at: string
  display: DisplayMoney
}

export type AmountKind = 'fixed' | 'ranged' | 'variation'

export interface CatalogueRequiredFields {
  credit_party_identifier?: string[]
  beneficiary?: string[]
  additional_identifier?: string[]
}

export interface CatalogueVariationItem {
  variation_code: string
  name: string
  amount: string
}

export interface CatalogueProduct {
  id: string
  slug: string
  name: string
  descriptor: string
  icon_key: string
  service_ref_label: string
  service_ref_placeholder: string
  service_ref_pattern: string | null
  quick_amounts: number[] | null
  amount_kind: AmountKind
  amount_min: string | null
  amount_max: string | null
  amount_increment: string | null
  currency: string
  fulfilment_kind: 'pin' | 'payment'
  operator_id: string | null
  operator_name: string | null
  required_fields: CatalogueRequiredFields
  field_labels: Record<string, string>
  variations: CatalogueVariationItem[]
}

export interface CatalogueCategory {
  category: string
  products: CatalogueProduct[]
}

export interface CatalogueCountry {
  iso2: string
  name: string
  currency: string
  categories: CatalogueCategory[]
}

export interface CatalogueResponse {
  countries: CatalogueCountry[]
}
