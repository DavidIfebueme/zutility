"use client"

import * as React from "react"
import Link from "next/link"
import { useParams, useRouter, useSearchParams } from "next/navigation"
import { QRCodeSVG } from "qrcode.react"
import { AlertTriangle, ArrowLeft, Check, Clock, Gift, Loader2, Share2, TimerOff, X } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { CopyField } from "@/components/ui/copy-field"
import { Stepper } from "@/components/ui/stepper"
import { CountdownTimer } from "@/components/ui/countdown-timer"
import { ConfirmationProgress } from "@/components/ui/confirmation-progress"
import { useOrderStore } from "@/store/order"
import { useAuthStore } from "@/store/auth"
import { useOrderStream } from "@/lib/hooks/useOrderStream"
import { apiGet, apiPost } from "@/lib/api"
import { formatZEC } from "@/lib/utils"
import { UTILITIES } from "@/lib/constants"
import { getStatusMeta, STATUS_TONE_CLASS } from "@/lib/order-status"
import type { CreateOrderResponse, OrderStatusResponse } from "@/lib/types"

function getCompletionInfo(slug: string): { title: string; description: string; tokenLabel: string } {
  const utility = UTILITIES.find((item) => item.slug === slug)
  if (!utility) {
    return {
      title: "Order delivered",
      description: "The utility has been delivered.",
      tokenLabel: "Token or PIN",
    }
  }

  switch (utility.type) {
    case "airtime":
      return {
        title: "Airtime delivered",
        description: "The phone number has been topped up.",
        tokenLabel: "Reference",
      }
    case "data":
      return {
        title: "Data bundle activated",
        description: "The bundle is active on the number you provided.",
        tokenLabel: "Reference",
      }
    case "tv":
      return {
        title: "Subscription renewed",
        description: "The TV subscription has been renewed.",
        tokenLabel: "Reference",
      }
    case "electricity":
      return {
        title: "Meter token ready",
        description: "Enter the token on the meter to load the units.",
        tokenLabel: "Meter token",
      }
    case "education":
      return {
        title: "PIN delivered",
        description: utility.name.includes("JAMB")
          ? "The JAMB registration PIN has been delivered."
          : "The exam PIN has been delivered. Use it on the official portal.",
        tokenLabel: "PIN",
      }
    case "school":
      return {
        title: "School fees paid",
        description: "The school fees payment has been confirmed.",
        tokenLabel: "Payment reference",
      }
    default:
      return {
        title: "Order delivered",
        description: "The utility has been delivered.",
        tokenLabel: "Token or PIN",
      }
  }
}

function fromStatus(status: OrderStatusResponse, token: string): CreateOrderResponse {
  return {
    order_id: status.order_id,
    order_access_token: token,
    deposit_address: status.deposit_address,
    zec_amount: status.zec_amount,
    expires_at: status.expires_at,
    qr_data: status.qr_data,
    required_confirmations: status.required_confirmations,
    utility_slug: status.utility_slug,
  }
}

async function tryNativeShare(url: string): Promise<boolean> {
  if (typeof navigator === "undefined" || !navigator.share) return false

  try {
    await navigator.share({ title: "A utility gift from zutility", url })
    return true
  } catch {
    return false
  }
}

export default function OrderPage() {
  return (
    <React.Suspense fallback={null}>
      <OrderPageContent />
    </React.Suspense>
  )
}

function OrderPageContent() {
  const params = useParams()
  const router = useRouter()
  const searchParams = useSearchParams()
  const orderId = params.orderId as string

  const { user } = useAuthStore()
  const { activeOrder, activeOrderUserId, clearActiveOrder } = useOrderStore()
  const [recoveredOrder, setRecoveredOrder] = React.useState<CreateOrderResponse | null>(null)
  const requestedOrder = React.useRef(false)

  const isCurrentOrder = activeOrder?.order_id === orderId && activeOrderUserId === user?.id
  const order = isCurrentOrder ? activeOrder : recoveredOrder

  const giftCode = searchParams.get("gift")
  const giftPath = giftCode ? `/gift/${giftCode}` : null
  const tokenParam = searchParams.get("token")
  const recoverParam = searchParams.get("recover")

  React.useEffect(() => {
    if (isCurrentOrder || recoveredOrder || requestedOrder.current || !orderId) return

    const token = tokenParam
    const recover = recoverParam

    if (token) {
      requestedOrder.current = true
      apiGet<OrderStatusResponse>(`/api/v1/orders/${orderId}?token=${encodeURIComponent(token)}`)
        .then((status) => setRecoveredOrder(fromStatus(status, token)))
        .catch(() => {
          requestedOrder.current = false
          toast.error("Could not load the order")
        })
      return
    }

    if (recover && user) {
      requestedOrder.current = true
      apiPost<{ order_access_token: string }>(`/api/v1/orders/${orderId}/access-token`, {})
        .then(async ({ order_access_token }) => {
          const status = await apiGet<OrderStatusResponse>(
            `/api/v1/orders/${orderId}?token=${encodeURIComponent(order_access_token)}`
          )
          setRecoveredOrder(fromStatus(status, order_access_token))
        })
        .catch(() => {
          requestedOrder.current = false
          toast.error("Could not load the order")
        })
    }
  }, [isCurrentOrder, recoveredOrder, orderId, tokenParam, recoverParam, user])

  const { status, confirmations, latestEvent, isConnected } = useOrderStream(
    order ? orderId : null,
    order ? order.order_access_token : null
  )

  if (!order) {
    return (
      <div className="flex flex-col items-center py-24 text-center">
        <AlertTriangle className="h-6 w-6 text-rust" strokeWidth={1.6} />
        <h2 className="mt-4 text-2xl text-ink">Order not found</h2>
        <p className="mt-2 max-w-sm text-[13px] text-muted">
          This order does not exist, or this browser does not have access to it. Open it from the
          link you were given, or from your order history.
        </p>
        <Button className="mt-6" onClick={() => router.push("/dashboard")}>
          Back to overview
        </Button>
      </div>
    )
  }

  const utility = UTILITIES.find((item) => item.slug === order.utility_slug)
  const meta = getStatusMeta(status)
  const steps = [
    { label: "Awaiting payment" },
    { label: "Confirming" },
    { label: "Dispatching" },
    { label: "Delivered" },
  ]

  const stepIndex = (() => {
    switch (status) {
      case "payment_detected":
        return 1
      case "payment_confirmed":
        return 2
      case "awaiting_claim":
        return 1
      case "utility_dispatching":
        return 2
      case "completed":
        return 4
      case "failed":
        return 2
      default:
        return 0
    }
  })()

  const addressType = order.required_confirmations > 3 ? "shielded" : "transparent"
  const isClosed = ["completed", "expired", "failed", "cancelled"].includes(status)

  const handleExpire = () => {
    if (status === "awaiting_payment") {
      toast.error("The order expired")
    }
  }

  const shareGift = async () => {
    if (!giftPath) return
    const url = `${window.location.origin}${giftPath}`
    const shared = await tryNativeShare(url)
    if (shared) return
    await navigator.clipboard.writeText(url)
    toast.success("Gift link copied")
  }

  return (
    <div className="mx-auto max-w-3xl space-y-5">
      <Link
        href="/dashboard"
        className="inline-flex items-center gap-1.5 text-[12px] text-muted transition-colors hover:text-ink"
      >
        <ArrowLeft className="h-3.5 w-3.5" />
        Back to overview
      </Link>

      <div className="rounded-lg border border-line bg-cream">
        <div className="flex flex-col gap-3 border-b border-line px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="label-xs text-muted">Receipt</p>
            <h1 className="mt-1 text-2xl text-ink">{utility?.name ?? order.utility_slug}</h1>
            <p className="num mt-0.5 text-[11px] text-muted">{order.order_id}</p>
          </div>
          <div className="flex flex-col items-start gap-1 sm:items-end">
            <span
              className={`inline-flex rounded-full border px-2.5 py-0.5 text-[11px] ${STATUS_TONE_CLASS[meta.tone]}`}
            >
              {meta.label}
            </span>
            <span className="text-[11px] capitalize text-muted">
              {addressType} address
              {!isConnected && !isClosed ? " · reconnecting" : ""}
            </span>
          </div>
        </div>

        <div className="border-b border-line px-5 py-4">
          <Stepper steps={steps} currentStep={stepIndex} />
        </div>

        {giftPath && (status === "awaiting_payment" || status === "awaiting_claim") && (
          <div className="border-b border-line bg-paper-2/60 px-5 py-4">
            <p className="flex items-center gap-1.5 text-[12px] font-medium text-gold-deep">
              <Gift className="h-3.5 w-3.5" />
              Gift link
            </p>
            <p className="mt-1 text-[12px] leading-relaxed text-muted">
              Share this link. The recipient enters their own account, and the first claim wins.
            </p>
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <code className="min-w-0 flex-1 truncate rounded-md border border-line bg-paper px-2.5 py-1.5 text-[11px] text-ink-2">
                {giftPath}
              </code>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  navigator.clipboard.writeText(`${window.location.origin}${giftPath}`)
                  toast.success("Gift link copied")
                }}
              >
                Copy
              </Button>
              <Button size="sm" onClick={shareGift}>
                <Share2 className="mr-1.5 h-3.5 w-3.5" />
                Share
              </Button>
            </div>
          </div>
        )}

        {status === "awaiting_payment" && (
          <div className="grid gap-6 p-5 md:grid-cols-[200px_1fr] md:gap-8">
            <div className="flex flex-col items-center">
              <div className="rounded-md border border-line bg-paper p-3">
                <QRCodeSVG
                  value={order.qr_data}
                  size={168}
                  level="M"
                  bgColor="#fffdf7"
                  fgColor="#14130f"
                />
              </div>
              <p className="mt-3 text-center text-[11px] leading-relaxed text-muted">
                Scan with Zashi, Nighthawk, Zingo, or any Zcash wallet.
              </p>
              <p className="mt-2 flex items-center gap-1.5 text-[12px] text-muted">
                <Clock className="h-3.5 w-3.5" />
                Expires in
                <CountdownTimer expiresAt={order.expires_at} onExpire={handleExpire} />
              </p>
            </div>

            <div className="min-w-0 space-y-5">
              <div className="space-y-2">
                <CopyField label="Amount to send" value={`${formatZEC(order.zec_amount)} ZEC`} />
                <CopyField label="Deposit address" value={order.deposit_address} />
              </div>

              <div className="rounded-md border border-line bg-paper px-4 py-3">
                <p className="flex items-center gap-1.5 text-[12px] font-medium text-gold-deep">
                  <AlertTriangle className="h-3.5 w-3.5" />
                  Before you send
                </p>
                <ul className="mt-2 space-y-1.5 text-[12px] leading-relaxed text-ink-2">
                  <li>Send exactly the quoted amount. A short payment is not credited.</li>
                  <li>The sender pays the network fee. Do not subtract it from the amount.</li>
                  <li>
                    Delivery follows {order.required_confirmations} confirmations, about four
                    minutes.
                  </li>
                </ul>
              </div>
            </div>
          </div>
        )}

        {(status === "payment_detected" || status === "payment_confirmed") && (
          <div className="space-y-5 p-6">
            <ConfirmationProgress
              current={confirmations}
              required={order.required_confirmations}
              addressType={addressType}
            />
            <div>
              <p className="text-lg text-ink">
                {status === "payment_confirmed" ? "Payment confirmed" : "Confirming payment"}
              </p>
              <p className="mt-1 max-w-lg text-[13px] leading-relaxed text-ink-2">
                {status === "payment_confirmed"
                  ? "The payment is final on the Zcash network. The utility is being dispatched now."
                  : `The payment reached the deposit address. We are waiting for ${order.required_confirmations} confirmations before dispatch.`}
              </p>
            </div>
          </div>
        )}

        {status === "awaiting_claim" && (
          <div className="space-y-3 p-6">
            <div className="flex items-start gap-2.5">
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-gold/10">
                <Gift className="h-4 w-4 text-gold-deep" />
              </span>
              <div>
                <p className="text-lg text-ink">Gift paid. Waiting to be claimed.</p>
                <p className="mt-1 max-w-lg text-[13px] leading-relaxed text-ink-2">
                  The payment is confirmed. Send the link to the recipient. When they claim it, the
                  utility is delivered to the account they enter.
                </p>
              </div>
            </div>
          </div>
        )}

        {status === "utility_dispatching" && (
          <div className="flex flex-col items-start gap-3 p-6">
            <Loader2 className="h-5 w-5 animate-spin text-gold-deep" />
            <p className="text-lg text-ink">Dispatching</p>
            <p className="max-w-lg text-[13px] leading-relaxed text-ink-2">
              The provider is processing the order. This usually takes less than a minute. Keep
              this page open or come back from the history page.
            </p>
          </div>
        )}

        {status === "completed" && (() => {
          const info = getCompletionInfo(order.utility_slug || "")
          return (
            <div className="space-y-5 p-6">
              <div className="flex items-start gap-2.5">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-green/10">
                  <Check className="h-4 w-4 text-green" />
                </span>
                <div>
                  <p className="text-lg text-ink">{info.title}</p>
                  <p className="mt-1 text-[13px] leading-relaxed text-ink-2">{info.description}</p>
                </div>
              </div>

              {latestEvent?.event === "completed" && latestEvent.delivery_token && (
                <div className="rounded-md border border-line bg-paper px-4 py-4">
                  <p className="label-xs text-muted">{info.tokenLabel}</p>
                  <p className="num mt-2 select-all break-all text-2xl tracking-wide text-ink">
                    {latestEvent.delivery_token}
                  </p>
                  <CopyField value={latestEvent.delivery_token} className="mt-3" />
                </div>
              )}

              <div className="flex flex-wrap gap-2 border-t border-line pt-4">
                <Button onClick={() => router.push("/dashboard")}>Back to overview</Button>
                <Button
                  variant="secondary"
                  onClick={() => {
                    clearActiveOrder()
                    router.push("/pay")
                  }}
                >
                  Pay another bill
                </Button>
                <Button asChild variant="ghost">
                  <Link href="/support">Get help</Link>
                </Button>
              </div>
            </div>
          )
        })()}

        {status === "expired" && (
          <div className="space-y-5 p-6">
            <div className="flex items-start gap-2.5">
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-paper-3">
                <TimerOff className="h-4 w-4 text-muted" />
              </span>
              <div>
                <p className="text-lg text-ink">Order expired</p>
                <p className="mt-1 max-w-lg text-[13px] leading-relaxed text-ink-2">
                  No payment arrived before the quote expired. Create a new order for a fresh rate
                  and a new deposit address.
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2 border-t border-line pt-4">
              <Button
                onClick={() => {
                  clearActiveOrder()
                  router.push("/pay")
                }}
              >
                Create a new order
              </Button>
              <Button variant="secondary" onClick={() => router.push("/dashboard")}>
                Back to overview
              </Button>
            </div>
          </div>
        )}

        {(status === "failed" || status === "cancelled") && (
          <div className="space-y-5 p-6">
            <div className="flex items-start gap-2.5">
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-rust/10">
                <X className="h-4 w-4 text-rust" />
              </span>
              <div>
                <p className="text-lg text-ink">
                  {status === "cancelled" ? "Order cancelled" : "Order failed"}
                </p>
                <p className="mt-1 max-w-lg text-[13px] leading-relaxed text-ink-2">
                  The utility could not be delivered. If ZEC left your wallet, open a support
                  request with the order ID and the transaction ID.
                </p>
              </div>
            </div>
            <div className="flex flex-wrap gap-2 border-t border-line pt-4">
              <Button asChild>
                <Link href="/support">Contact support</Link>
              </Button>
              <Button
                variant="secondary"
                onClick={() => {
                  clearActiveOrder()
                  router.push("/pay")
                }}
              >
                Try another order
              </Button>
            </div>
          </div>
        )}
      </div>

      <p className="px-1 text-[11px] leading-relaxed text-muted">
        Keep this page open while the payment confirms. The status updates over a live connection.
        Support can trace any order by its ID.
      </p>
    </div>
  )
}
