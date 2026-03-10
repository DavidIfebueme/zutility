export type OrderStatus =
  | 'awaiting_payment' | 'payment_detected' | 'payment_confirmed'
  | 'utility_dispatching' | 'completed' | 'expired' | 'failed' | 'flagged_for_review'

export interface CreateOrderRequest {
  utility_type: string
  utility_slug: string
  service_ref: string
  amount_ngn: number            // kobo (NGN × 100)
  zec_address_type: 'shielded' | 'transparent'
}

export interface CreateOrderResponse {
  order_id: string
  order_access_token: string    // store in localStorage immediately — shown once
  deposit_address: string
  zec_amount: string            // always string — never parseFloat ZEC amounts
  expires_at: string
  qr_data: string               // ZEC URI: zcash:ADDRESS?amount=X
  required_confirmations: number
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

export type OrderStreamEvent =
  | { event: 'payment_detected'; confirmations: number; required: number }
  | { event: 'confirmation'; confirmations: number; required: number }
  | { event: 'payment_confirmed'; confirmations: number }
  | { event: 'dispatching' }
  | { event: 'completed'; delivery_token: string | null; reference: string }
  | { event: 'expired' }
  | { event: 'failed'; reason: string }
