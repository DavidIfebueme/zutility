"use client"

import * as React from "react"
import Link from "next/link"
import { ArrowRight, Loader2, Receipt } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useAuthStore } from "@/store/auth"
import { useOrderStore } from "@/store/order"
import { useRate } from "@/lib/hooks/useRate"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { useFxRates } from "@/lib/hooks/useFxRates"
import { apiGet } from "@/lib/api"
import { formatNGN, formatZEC } from "@/lib/utils"
import {
  convertFromNGN,
  formatCurrency,
  formatLocalAmount,
  type CurrencyCode,
  type FxRates,
} from "@/lib/currency"
import { getStatusMeta, STATUS_TONE_CLASS } from "@/lib/order-status"

interface OrderHistoryItem {
  order_id: string
  utility_slug: string
  utility_type: string
  amount_ngn: number
  zec_amount: string
  status: string
  created_at: string
  completed_at: string | null
}

function rateForCurrency(rate: NonNullable<ReturnType<typeof useRate>["rate"]>, currency: CurrencyCode): number {
  const fx: FxRates = {
    usd_ngn: parseFloat(rate.usd_ngn) || 1,
    usd_kes: parseFloat(rate.usd_kes) || 0,
    usd_ghs: parseFloat(rate.usd_ghs) || 0,
    usd_zar: parseFloat(rate.usd_zar) || 0,
    usd_egp: parseFloat(rate.usd_egp) || 0,
  }
  if (currency === "NGN") return parseFloat(rate.zec_ngn)
  if (currency === "USD") return parseFloat(rate.zec_usd)
  return convertFromNGN(parseFloat(rate.zec_ngn), currency, fx)
}

export default function DashboardPage() {
  const { user } = useAuthStore()
  const { activeOrder: storedOrder, activeOrderUserId } = useOrderStore()
  const activeOrder = storedOrder && activeOrderUserId === user?.id ? storedOrder : null
  const { rate } = useRate()
  const currency = useCurrency()
  const fxRates = useFxRates()
  const [orders, setOrders] = React.useState<OrderHistoryItem[]>([])
  const [ordersLoading, setOrdersLoading] = React.useState(true)

  React.useEffect(() => {
    apiGet<OrderHistoryItem[]>("/api/v1/orders/history")
      .then(setOrders)
      .catch(() => setOrders([]))
      .finally(() => setOrdersLoading(false))
  }, [])

  const recent = orders.slice(0, 5)
  const activeStatus = activeOrder
    ? (orders.find((order) => order.order_id === activeOrder.order_id)?.status ?? "awaiting_payment")
    : "awaiting_payment"
  const activeMeta = getStatusMeta(activeStatus)
  const activeIsPaid = activeStatus !== "awaiting_payment"

  return (
    <div className="space-y-9">
      <header>
        <p className="label-xs text-muted">Overview</p>
        <h1 className="mt-2 text-3xl text-ink">
          {user?.display_name ? `Hello, ${user.display_name}.` : "Hello."}
        </h1>
        <p className="mt-1.5 text-[14px] text-muted">
          Pay a bill, follow an open order, or check what you paid last month.
        </p>
      </header>

      {activeOrder ? (
        <section className="rounded-lg border border-line bg-cream">
          <div className="flex items-center justify-between border-b border-line px-5 py-3">
            <p className="label-xs text-muted">Open order</p>
            <span
              className={`inline-flex rounded-full border px-2.5 py-0.5 text-[11px] ${STATUS_TONE_CLASS[activeMeta.tone]}`}
            >
              {activeMeta.label}
            </span>
          </div>
          <div className="flex flex-col gap-5 p-5 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <p className="num text-[11px] text-muted">{activeOrder.order_id}</p>
              <p className="mt-2 text-2xl text-ink">
                <span className="num">{formatZEC(activeOrder.zec_amount)} ZEC</span>
              </p>
              <p className="mt-1 text-[13px] text-ink-2">
                {activeIsPaid
                  ? "This order is paid and in progress. Open it for the latest status."
                  : "Send this amount to the deposit address on the order page. The quote holds until the order expires."}
              </p>
            </div>
            <Button asChild className="shrink-0">
              <Link href={`/pay/${activeOrder.order_id}`}>
                View order
                <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" />
              </Link>
            </Button>
          </div>
        </section>
      ) : (
        <section className="rounded-lg border border-line bg-cream">
          <div className="flex flex-col gap-5 p-5 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <p className="label-xs text-muted">Start here</p>
              <p className="mt-2 text-2xl text-ink">Pay a bill with ZEC.</p>
              <p className="mt-1 max-w-md text-[13px] leading-relaxed text-ink-2">
                Airtime, data, TV, prepaid electricity, exam PINs, and school fees. The order gets
                one address and one amount.
              </p>
            </div>
            <Button asChild className="shrink-0">
              <Link href="/pay">
                Pay a bill
                <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/btn:translate-x-0.5" />
              </Link>
            </Button>
          </div>
        </section>
      )}

      <div className="grid gap-8 lg:grid-cols-[1fr_248px]">
        <section className="min-w-0">
          <div className="flex items-baseline justify-between">
            <h2 className="text-lg text-ink">Recent orders</h2>
            <Link
              href="/history"
              className="text-[12px] text-gold-deep u-link"
            >
              All orders
            </Link>
          </div>

          {ordersLoading ? (
            <div className="mt-3 flex justify-center rounded-lg border border-line bg-cream py-12">
              <Loader2 className="h-4 w-4 animate-spin text-muted" />
            </div>
          ) : recent.length === 0 ? (
            <div className="mt-3 flex flex-col items-center rounded-lg border border-dashed border-line-strong bg-cream/60 px-6 py-12 text-center">
              <Receipt className="h-5 w-5 text-muted" strokeWidth={1.6} />
              <p className="mt-3 text-[14px] text-ink">No orders yet</p>
              <p className="mt-1 max-w-xs text-[13px] text-muted">
                Your first receipt will show up here.
              </p>
            </div>
          ) : (
            <div className="mt-3 overflow-hidden rounded-lg border border-line bg-cream">
              <table className="w-full text-left">
                <thead>
                  <tr className="border-b border-line">
                    <th className="label-xs px-4 py-2.5 font-medium text-muted">Service</th>
                    <th className="label-xs hidden px-4 py-2.5 font-medium text-muted sm:table-cell">
                      Date
                    </th>
                    <th className="label-xs px-4 py-2.5 text-right font-medium text-muted">Amount</th>
                    <th className="label-xs px-4 py-2.5 text-right font-medium text-muted">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {recent.map((order) => {
                    const meta = getStatusMeta(order.status)
                    return (
                      <tr
                        key={order.order_id}
                        className="border-b border-line last:border-b-0 hover:bg-paper-2/60"
                      >
                        <td className="px-4 py-3">
                          <Link
                            href={`/pay/${order.order_id}?recover=1`}
                            className="block text-[13px] text-ink hover:underline"
                          >
                            {order.utility_slug}
                          </Link>
                          <span className="num text-[11px] text-muted">
                            {order.order_id.slice(0, 8)}
                          </span>
                        </td>
                        <td className="num hidden px-4 py-3 text-[12px] text-muted sm:table-cell">
                          {new Date(order.created_at).toLocaleDateString()}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <span className="num block text-[13px] text-ink">
                            {fxRates ? formatLocalAmount(order.amount_ngn, currency, fxRates) : "—"}
                          </span>
                          <span className="num block text-[11px] text-muted">
                            {formatZEC(order.zec_amount)} ZEC
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right">
                          <span
                            className={`inline-flex rounded-full border px-2 py-0.5 text-[11px] ${STATUS_TONE_CLASS[meta.tone]}`}
                          >
                            {meta.label}
                          </span>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
        </section>

        <aside className="space-y-5">
          <div className="rounded-lg border border-line bg-cream p-4">
            <p className="label-xs text-muted">ZEC / {currency}</p>
            <p className="num mt-2 text-xl text-ink">
              {rate ? formatCurrency(rateForCurrency(rate, currency), currency) : "---"}
            </p>
            <p className="mt-1 text-[11px] text-muted">
              {rate?.updated_at
                ? `Updated ${new Date(rate.updated_at).toLocaleTimeString()}`
                : "Waiting for the feed"}
            </p>
            <p className="mt-3 border-t border-line pt-3 text-[11px] leading-relaxed text-muted">
              Orders keep the rate they were quoted at, even if the market moves.
            </p>
          </div>

          <div className="rounded-lg border border-line bg-paper-2 p-4">
            <p className="label-xs text-muted">Later</p>
            <ul className="mt-2 space-y-2">
              <li className="flex items-baseline justify-between gap-2 text-[13px] text-ink-2">
                OTC off-ramp
                <span className="text-[10px] uppercase tracking-wider text-muted">Soon</span>
              </li>
              <li className="flex items-baseline justify-between gap-2 text-[13px] text-ink-2">
                P2P market
                <span className="text-[10px] uppercase tracking-wider text-muted">Soon</span>
              </li>
            </ul>
          </div>
        </aside>
      </div>
    </div>
  )
}
