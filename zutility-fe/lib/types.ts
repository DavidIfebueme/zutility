export type OrderStatus =
  | 'awaiting_payment' | 'payment_detected' | 'payment_confirmed' | 'awaiting_claim'
  | 'utility_dispatching' | 'completed' | 'expired' | 'failed' | 'flagged_for_review' | 'cancelled'

export interface CreateOrderRequest {
  utility_type: string
  utility_slug: string
  service_ref: string
  amount_ngn: number
  zec_address_type: 'shielded' | 'transparent'
  variation_code?: string
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
  | { event: 'cancelled' }

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
}
