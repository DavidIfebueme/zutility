"use client"

import * as React from "react"
import Link from "next/link"
import { AlertTriangle, Check, Gift, Loader2, TimerOff } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { CopyField } from "@/components/ui/copy-field"
import { BrandMark } from "@/components/ui/brand-mark"
import { WordmarkLink } from "@/components/ui/wordmark"
import { getApiBaseUrl } from "@/lib/api"
import { UTILITIES } from "@/lib/constants"
import { formatNGN, formatZEC } from "@/lib/utils"
import type { GiftView, OrderStatus } from "@/lib/types"

const apiBase = getApiBaseUrl()

async function giftRequest<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBase}${path}`, {
    cache: "no-store",
    ...init,
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
  })
  if (!response.ok) {
    let message = `Request failed with status ${response.status}`
    try {
      const body: { error?: unknown } = await response.json()
      if (typeof body.error === "string") message = body.error
    } catch {
      message = `Request failed with status ${response.status}`
    }
    throw new Error(message)
  }
  return response.json() as Promise<T>
}

const TERMINAL_ORDER_STATUSES: OrderStatus[] = [
  "completed",
  "failed",
  "expired",
  "cancelled",
  "flagged_for_review",
]

function deliveryHeadline(status: OrderStatus | null): string {
  switch (status) {
    case "awaiting_payment":
      return "Waiting for the sender to pay"
    case "payment_detected":
    case "payment_confirmed":
      return "Payment confirming"
    case "utility_dispatching":
      return "Delivering now"
    case "completed":
      return "Delivered"
    case "failed":
      return "Delivery failed"
    case "flagged_for_review":
      return "Under review"
    case "expired":
      return "The sender did not pay in time"
    case "cancelled":
      return "This order was cancelled"
    default:
      return "Claimed"
  }
}

export function GiftClaim({ code, initialGift }: { code: string; initialGift: GiftView | null }) {
  const [gift, setGift] = React.useState<GiftView | null>(initialGift)
  const [serviceRef, setServiceRef] = React.useState("")
  const [claiming, setClaiming] = React.useState(false)
  const [missing, setMissing] = React.useState(initialGift === null)

  const utility = gift ? UTILITIES.find((item) => item.slug === gift.utility_slug) : undefined

  const refresh = React.useCallback(async () => {
    try {
      const next = await giftRequest<GiftView>(`/api/v1/gifts/${encodeURIComponent(code)}`)
      setGift(next)
      setMissing(false)
    } catch {
      setMissing(true)
    }
  }, [code])

  React.useEffect(() => {
    if (!gift || gift.status !== "claimed") return
    if (gift.order_status && TERMINAL_ORDER_STATUSES.includes(gift.order_status)) return

    const timer = window.setInterval(refresh, 4000)
    return () => window.clearInterval(timer)
  }, [gift, refresh])

  const claim = async (event: React.FormEvent) => {
    event.preventDefault()
    if (serviceRef.trim().length < 5) {
      toast.error("Enter a valid account number")
      return
    }

    setClaiming(true)
    try {
      const next = await giftRequest<GiftView>(`/api/v1/gifts/${encodeURIComponent(code)}/claim`, {
        method: "POST",
        body: JSON.stringify({ service_ref: serviceRef.trim() }),
      })
      setGift(next)
      toast.success("Gift claimed")
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Could not claim the gift")
    } finally {
      setClaiming(false)
    }
  }

  return (
    <div className="min-h-screen bg-paper">
      <header className="border-b border-line">
        <div className="mx-auto flex h-16 max-w-lg items-center justify-between px-5">
          <WordmarkLink />
          <Link href="/" className="text-[12px] text-muted transition-colors hover:text-ink">
            What is zutility?
          </Link>
        </div>
      </header>

      <main className="mx-auto w-full max-w-lg px-5 py-10">
        {missing && !gift && (
          <div className="rounded-xl border border-line bg-cream p-8 text-center">
            <AlertTriangle className="mx-auto h-6 w-6 text-rust" strokeWidth={1.6} />
            <h1 className="mt-4 text-2xl text-ink">Gift not found</h1>
            <p className="mt-2 text-[13px] leading-relaxed text-muted">
              This link is wrong or the gift does not exist. Ask the sender for a fresh link.
            </p>
            <Button variant="secondary" className="mt-5" onClick={refresh}>
              Try again
            </Button>
          </div>
        )}

        {gift && (
          <div className="overflow-hidden rounded-xl border border-line bg-cream">
            <div className="border-b border-line bg-paper-2 px-6 py-5">
              <p className="label-xs flex items-center gap-1.5 text-gold-deep">
                <Gift className="h-3.5 w-3.5" />
                A utility gift
              </p>
              <h1 className="mt-2 text-2xl leading-tight text-ink">
                {gift.sender_name} sent you{" "}
                {gift.zec_amount ? `${formatZEC(gift.zec_amount)} ZEC` : formatNGN(gift.amount_ngn)}
              </h1>
              <p className="mt-1 text-[13px] text-muted">
                {utility?.name ?? gift.utility_slug}
              </p>
            </div>

            <div className="px-6 py-6">
              <div className="flex items-center gap-3">
                <span
                  className="flex h-12 w-12 items-center justify-center rounded-lg bg-paper-2"
                  aria-hidden
                >
                  {utility ? <BrandMark id={utility.brandIcon as never} size={30} /> : <Gift className="h-6 w-6 text-gold-deep" />}
                </span>
                <div className="min-w-0">
                  <p className="text-[13px] font-medium text-ink">{utility?.name ?? gift.utility_slug}</p>
                  <p className="num text-[12px] text-muted">
                    {gift.zec_amount ? `${formatZEC(gift.zec_amount)} ZEC` : formatNGN(gift.amount_ngn)}
                  </p>
                </div>
              </div>

              {gift.message && (
                <blockquote className="mt-5 border-l-2 border-gold/50 pl-3 text-[13px] leading-relaxed text-ink-2">
                  {gift.message}
                </blockquote>
              )}

              {gift.status === "open" && (
                <form onSubmit={claim} className="mt-6 space-y-3">
                  <label htmlFor="serviceRef" className="block text-[13px] font-medium text-ink">
                    {utility?.serviceRefLabel ?? "Account number"}
                  </label>
                  <Input
                    id="serviceRef"
                    value={serviceRef}
                    onChange={(event) => setServiceRef(event.target.value)}
                    placeholder={utility?.serviceRefPlaceholder ?? "Enter the account number"}
                    autoComplete="off"
                  />
                  <Button type="submit" className="w-full" size="lg" loading={claiming}>
                    Claim this gift
                  </Button>
                  <p className="text-[11px] leading-relaxed text-muted">
                    The gift is delivered to this account. Check the number before you claim.
                  </p>
                </form>
              )}

              {gift.status === "claimed" && (
                <div className="mt-6 space-y-4">
                  <div className="flex items-center gap-2">
                    {gift.order_status === "completed" ? (
                      <span className="flex h-7 w-7 items-center justify-center rounded-full bg-green/10">
                        <Check className="h-4 w-4 text-green" />
                      </span>
                    ) : gift.order_status === "failed" || gift.order_status === "flagged_for_review" ? (
                      <span className="flex h-7 w-7 items-center justify-center rounded-full bg-rust/10">
                        <AlertTriangle className="h-4 w-4 text-rust" />
                      </span>
                    ) : (
                      <Loader2 className="h-5 w-5 animate-spin text-gold-deep" />
                    )}
                    <div>
                      <p className="text-[14px] text-ink">{deliveryHeadline(gift.order_status)}</p>
                      <p className="text-[12px] text-muted">
                        {gift.order_status === "awaiting_payment"
                          ? "The sender has not paid yet. This page updates on its own."
                          : gift.order_status === "completed"
                            ? "The utility has been delivered to the account you entered."
                            : gift.order_status === "expired" || gift.order_status === "cancelled"
                              ? "The order closed before delivery. Ask the sender to send a new gift."
                              : gift.order_status === "flagged_for_review"
                                ? "This order is being reviewed. Support will follow up."
                                : "Keep this page open. It updates on its own."}
                      </p>
                    </div>
                  </div>

                  {gift.order_status === "completed" && gift.delivery_token && (
                    <div className="rounded-md border border-line bg-paper px-4 py-4">
                      <p className="label-xs text-muted">Token</p>
                      <p className="num mt-2 select-all break-all text-xl tracking-wide text-ink">
                        {gift.delivery_token}
                      </p>
                      <CopyField value={gift.delivery_token} className="mt-3" />
                    </div>
                  )}
                </div>
              )}

              {gift.status === "expired" && (
                <div className="mt-6 flex items-start gap-2.5">
                  <TimerOff className="mt-0.5 h-5 w-5 shrink-0 text-muted" />
                  <div>
                    <p className="text-[14px] text-ink">This gift expired</p>
                    <p className="mt-1 text-[12px] leading-relaxed text-muted">
                      Nobody claimed it before the deadline. Ask the sender to send a new one.
                    </p>
                  </div>
                </div>
              )}

              {gift.status === "cancelled" && (
                <div className="mt-6 flex items-start gap-2.5">
                  <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-rust" />
                  <div>
                    <p className="text-[14px] text-ink">This gift was cancelled</p>
                    <p className="mt-1 text-[12px] leading-relaxed text-muted">
                      The sender cancelled it before it was claimed.
                    </p>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        <p className="mt-6 text-center text-[11px] leading-relaxed text-muted">
          zutility delivers the utility to the account you enter. No account is needed to claim.
        </p>
      </main>
    </div>
  )
}
