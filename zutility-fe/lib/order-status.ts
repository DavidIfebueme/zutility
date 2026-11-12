export type StatusTone = "success" | "warning" | "error" | "neutral"

export interface StatusMeta {
  key: string
  label: string
  tone: StatusTone
}

const STATUS: Record<string, StatusMeta> = {
  completed: { key: "completed", label: "Delivered", tone: "success" },
  payment_confirmed: { key: "payment_confirmed", label: "Payment confirmed", tone: "success" },
  awaiting_claim: { key: "awaiting_claim", label: "Waiting for claim", tone: "warning" },
  payment_detected: { key: "payment_detected", label: "Payment seen", tone: "warning" },
  utility_dispatching: { key: "utility_dispatching", label: "Dispatching", tone: "warning" },
  awaiting_payment: { key: "awaiting_payment", label: "Awaiting payment", tone: "neutral" },
  pending: { key: "pending", label: "Pending", tone: "neutral" },
  expired: { key: "expired", label: "Expired", tone: "neutral" },
  cancelled: { key: "cancelled", label: "Cancelled", tone: "neutral" },
  failed: { key: "failed", label: "Failed", tone: "error" },
  flagged_for_review: { key: "flagged_for_review", label: "In review", tone: "warning" },
}

export function normalizeStatus(raw: string): string {
  return raw.replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase()
}

export function getStatusMeta(raw: string): StatusMeta {
  const key = normalizeStatus(raw)
  return STATUS[key] ?? { key, label: raw, tone: "neutral" }
}

export const STATUS_TONE_CLASS: Record<StatusTone, string> = {
  success: "border-green/25 bg-green/8 text-green",
  warning: "border-gold-deep/25 bg-gold/15 text-gold-deep",
  error: "border-rust/25 bg-rust/8 text-rust",
  neutral: "border-line bg-paper-2 text-ink-2",
}
