"use client"

import * as React from "react"
import Link from "next/link"
import { Copy, Gift, Loader2, Share2, X } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { BrandMark } from "@/components/ui/brand-mark"
import { apiGet, apiPost } from "@/lib/api"
import { UTILITIES } from "@/lib/constants"
import { formatNGN } from "@/lib/utils"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { useFxRates } from "@/lib/hooks/useFxRates"
import { formatLocalAmount } from "@/lib/currency"
import type { GiftSummary } from "@/lib/types"

function giftState(gift: GiftSummary): { label: string; tone: string } {
  if (gift.status === "cancelled") return { label: "Cancelled", tone: "border-line text-muted" }
  if (gift.order_status === "cancelled") return { label: "Cancelled", tone: "border-line text-muted" }

  if (gift.order_status === "expired") {
    return gift.status === "claimed"
      ? { label: "Claimed, unpaid", tone: "border-rust/40 text-rust" }
      : { label: "Expired", tone: "border-line text-muted" }
  }

  if (gift.status === "expired") return { label: "Expired", tone: "border-line text-muted" }

  if (gift.status === "open") {
    if (gift.order_status === "awaiting_payment") return { label: "Unpaid", tone: "border-gold/40 text-gold-deep" }
    if (gift.order_status === "awaiting_claim") return { label: "Waiting to be claimed", tone: "border-gold/40 text-gold-deep" }
    return { label: "Open", tone: "border-gold/40 text-gold-deep" }
  }

  switch (gift.order_status) {
    case "awaiting_payment":
      return { label: "Claimed, unpaid", tone: "border-gold/40 text-gold-deep" }
    case "payment_detected":
    case "payment_confirmed":
      return { label: "Confirming", tone: "border-gold/40 text-gold-deep" }
    case "utility_dispatching":
      return { label: "Delivering", tone: "border-gold/40 text-gold-deep" }
    case "completed":
      return { label: "Delivered", tone: "border-green/40 text-green" }
    case "failed":
      return { label: "Failed", tone: "border-rust/40 text-rust" }
    case "flagged_for_review":
      return { label: "Under review", tone: "border-rust/40 text-rust" }
    default:
      return { label: "Claimed", tone: "border-line text-ink-2" }
  }
}

export default function GiftsPage() {
  const currency = useCurrency()
  const fxRates = useFxRates()
  const [gifts, setGifts] = React.useState<GiftSummary[]>([])
  const [loading, setLoading] = React.useState(true)
  const [working, setWorking] = React.useState<string | null>(null)
  const [reloadKey, setReloadKey] = React.useState(0)

  React.useEffect(() => {
    apiGet<GiftSummary[]>("/api/v1/gifts")
      .then(setGifts)
      .catch(() => toast.error("Could not load your gifts"))
      .finally(() => setLoading(false))
  }, [reloadKey])

  const generateLink = async (gift: GiftSummary) => {
    setWorking(gift.gift_id)
    try {
      const result = await apiPost<{ gift_code: string; gift_url: string }>(
        `/api/v1/gifts/${gift.gift_id}/rotate-code`,
        {}
      )
      await navigator.clipboard.writeText(result.gift_url)
      toast.success("New gift link copied. The old link no longer works.")
    } catch {
      toast.error("Could not create a new link")
    } finally {
      setWorking(null)
    }
  }

  const cancelGift = async (gift: GiftSummary) => {
    setWorking(gift.gift_id)
    try {
      await apiPost(`/api/v1/gifts/${gift.gift_id}/cancel`, {})
      toast.success("Gift cancelled")
      setReloadKey((value) => value + 1)
    } catch {
      toast.error("Could not cancel this gift")
    } finally {
      setWorking(null)
    }
  }

  return (
    <div className="space-y-7">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <p className="label-xs text-muted">Gifts</p>
          <h1 className="mt-2 text-3xl text-ink">Gifts you sent</h1>
          <p className="mt-1.5 max-w-lg text-[14px] leading-relaxed text-muted">
            A gift stays payable until it is claimed. If nobody claims it before the deadline, it is
            delivered to your backup account.
          </p>
        </div>
        <Button asChild>
          <Link href="/pay">Send a gift</Link>
        </Button>
      </header>

      {loading && (
        <div className="flex items-center gap-2 py-12 text-[13px] text-muted">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading gifts
        </div>
      )}

      {!loading && gifts.length === 0 && (
        <div className="rounded-lg border border-dashed border-line-strong px-6 py-14 text-center">
          <Gift className="mx-auto h-6 w-6 text-muted" strokeWidth={1.6} />
          <p className="mt-3 text-[15px] text-ink">No gifts yet</p>
          <p className="mx-auto mt-1 max-w-sm text-[13px] leading-relaxed text-muted">
            Pick a utility, add a message, and share the link. The recipient enters their own
            account, so you never need their number.
          </p>
          <Button asChild className="mt-5">
            <Link href="/pay">Create your first gift</Link>
          </Button>
        </div>
      )}

      <div className="space-y-2">
        {gifts.map((gift) => {
          const utility = UTILITIES.find((item) => item.slug === gift.utility_slug)
          const state = giftState(gift)
          const canShare = gift.status === "open"
          const canCancel =
            gift.status === "open" && gift.order_status === "awaiting_payment"
          const canPay = Boolean(gift.order_id) && gift.order_status === "awaiting_payment"
          const isTerminalOrder =
            !gift.order_status ||
            ["completed", "failed", "cancelled", "expired", "flagged_for_review"].includes(
              gift.order_status
            )
          const canOpen = Boolean(gift.order_id) && !isTerminalOrder
          const busy = working === gift.gift_id

          return (
            <div
              key={gift.gift_id}
              className="flex flex-col gap-3 rounded-lg border border-line bg-cream px-4 py-3 sm:flex-row sm:items-center"
            >
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-paper-2">
                {utility ? (
                  <BrandMark id={utility.brandIcon as never} size={24} />
                ) : (
                  <Gift className="h-5 w-5 text-gold-deep" />
                )}
              </span>

              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
                  <p className="truncate text-[14px] font-medium text-ink">
                    {utility?.name ?? gift.utility_slug}
                  </p>
                  <span className={`rounded-full border px-2 py-0.5 text-[10px] ${state.tone}`}>
                    {state.label}
                  </span>
                </div>
                <p className="num mt-0.5 text-[12px] text-muted">
                  {fxRates ? formatLocalAmount(gift.amount_ngn, currency, fxRates) : "—"}
                  {gift.claimed_service_ref ? ` · to ${gift.claimed_service_ref}` : ""}
                </p>
              </div>

              <div className="flex flex-wrap items-center gap-2">
                {canPay && (
                  <Button asChild size="sm">
                    <Link href={`/pay/${gift.order_id}?recover=1`}>Pay now</Link>
                  </Button>
                )}
                {canOpen && !canPay && (
                  <Button asChild size="sm" variant="secondary">
                    <Link href={`/pay/${gift.order_id}?recover=1`}>Open</Link>
                  </Button>
                )}
                {canShare && (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => generateLink(gift)}
                    loading={busy}
                  >
                    {busy ? null : <Share2 className="mr-1.5 h-3.5 w-3.5" />}
                    New link
                  </Button>
                )}
                {canCancel && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => cancelGift(gift)}
                    loading={busy}
                  >
                    {busy ? null : <X className="mr-1.5 h-3.5 w-3.5" />}
                    Cancel
                  </Button>
                )}
              </div>
            </div>
          )
        })}
      </div>

      {gifts.some((gift) => gift.status === "open") && (
        <p className="flex items-start gap-1.5 text-[11px] leading-relaxed text-muted">
          <Copy className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          Generating a new link stops the previous one. Only the newest link can be claimed.
        </p>
      )}
    </div>
  )
}
