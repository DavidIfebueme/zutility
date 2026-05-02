export type OrderStatus =
  | 'awaiting_payment' | 'payment_detected' | 'payment_confirmed'
  | 'utility_dispatching' | 'completed' | 'expired' | 'failed' | 'flagged_for_review'

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
  expires_at: string
  completed_at: string | null
  delivery_token: string | null
}

export interface RateResponse {
  zec_ngn: string
  zec_usd: string
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
}

export type OrderStreamEvent =
  | { event: 'payment_detected'; confirmations: number; required: number }
  | { event: 'confirmation'; confirmations: number; required: number }
  | { event: 'payment_confirmed'; confirmations: number }
  | { event: 'dispatching' }
  | { event: 'completed'; delivery_token: string | null; reference: string }
  | { event: 'expired' }
  | { event: 'failed'; reason: string }
