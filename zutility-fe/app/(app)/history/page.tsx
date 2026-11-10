"use client"

import * as React from "react"
import Link from "next/link"
import { ArrowRight, Download, Loader2, Receipt } from "lucide-react"
import { Button } from "@/components/ui/button"
import { apiGet } from "@/lib/api"
import { formatNGN, formatZEC } from "@/lib/utils"
import { getStatusMeta, STATUS_TONE_CLASS } from "@/lib/order-status"
import { cn } from "@/lib/utils"

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

const FILTERS = [
  { id: "all", label: "All" },
  { id: "open", label: "Open" },
  { id: "delivered", label: "Delivered" },
  { id: "failed", label: "Failed" },
] as const

type FilterId = (typeof FILTERS)[number]["id"]

function matchesFilter(status: string, filter: FilterId): boolean {
  const key = getStatusMeta(status).key
  if (filter === "all") return true
  if (filter === "delivered") return key === "completed"
  if (filter === "failed") return ["failed", "expired", "cancelled"].includes(key)
  return ["awaiting_payment", "payment_detected", "payment_confirmed", "utility_dispatching", "pending"].includes(key)
}

export default function HistoryPage() {
  const [orders, setOrders] = React.useState<OrderHistoryItem[]>([])
  const [loading, setLoading] = React.useState(true)
  const [filter, setFilter] = React.useState<FilterId>("all")

  React.useEffect(() => {
    apiGet<OrderHistoryItem[]>("/api/v1/orders/history")
      .then(setOrders)
      .catch(() => setOrders([]))
      .finally(() => setLoading(false))
  }, [])

  const visible = orders.filter((order) => matchesFilter(order.status, filter))

  return (
    <div className="space-y-7">
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div>
          <p className="label-xs text-muted">History</p>
          <h1 className="mt-2 text-3xl text-ink">Order history</h1>
          <p className="mt-1.5 text-[14px] text-muted">
            Every payment made from this account, newest first.
          </p>
        </div>
        <Button asChild variant="secondary" size="sm">
          <Link href="/pay">Pay a bill</Link>
        </Button>
      </header>

      <div className="flex flex-wrap items-center gap-1.5">
        {FILTERS.map((item) => (
          <button
            key={item.id}
            onClick={() => setFilter(item.id)}
            className={cn(
              "rounded-md border px-2.5 py-1 text-[12px] transition-colors",
              filter === item.id
                ? "border-line-strong bg-cream text-ink"
                : "border-transparent text-muted hover:text-ink"
            )}
          >
            {item.label}
          </button>
        ))}
        <span className="num ml-auto text-[11px] text-muted">
          {visible.length} of {orders.length}
        </span>
      </div>

      {loading ? (
        <div className="flex justify-center rounded-lg border border-line bg-cream py-20">
          <Loader2 className="h-4 w-4 animate-spin text-muted" />
        </div>
      ) : visible.length === 0 ? (
        <div className="flex flex-col items-center rounded-lg border border-dashed border-line-strong bg-cream/60 px-6 py-16 text-center">
          <Receipt className="h-5 w-5 text-muted" strokeWidth={1.6} />
          <p className="mt-3 text-[14px] text-ink">
            {orders.length === 0 ? "No orders yet" : "Nothing in this filter"}
          </p>
          <p className="mt-1 max-w-xs text-[13px] text-muted">
            {orders.length === 0
              ? "Start with a small top-up and the receipt will appear here."
              : "Try another filter to see the rest of your orders."}
          </p>
          {orders.length === 0 && (
            <Button asChild className="mt-5" size="sm">
              <Link href="/pay">
                Pay a bill
                <ArrowRight className="h-3.5 w-3.5" />
              </Link>
            </Button>
          )}
        </div>
      ) : (
        <div className="overflow-hidden rounded-lg border border-line bg-cream">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-line">
                <th className="label-xs px-4 py-2.5 font-medium text-muted">Service</th>
                <th className="label-xs hidden px-4 py-2.5 font-medium text-muted sm:table-cell">
                  Reference
                </th>
                <th className="label-xs hidden px-4 py-2.5 font-medium text-muted md:table-cell">
                  Date
                </th>
                <th className="label-xs px-4 py-2.5 text-right font-medium text-muted">Amount</th>
                <th className="label-xs px-4 py-2.5 text-right font-medium text-muted">Status</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((order) => {
                const meta = getStatusMeta(order.status)
                return (
                  <tr
                    key={order.order_id}
                    className="border-b border-line last:border-b-0 hover:bg-paper-2/60"
                  >
                    <td className="px-4 py-3">
                      <Link
                        href={`/pay/${order.order_id}`}
                        className="text-[13px] text-ink hover:underline"
                      >
                        {order.utility_slug}
                      </Link>
                      <span className="block text-[11px] text-muted sm:hidden">
                        {new Date(order.created_at).toLocaleDateString()}
                      </span>
                    </td>
                    <td className="num hidden px-4 py-3 text-[12px] text-muted sm:table-cell">
                      {order.order_id.slice(0, 8)}
                    </td>
                    <td className="num hidden px-4 py-3 text-[12px] text-muted md:table-cell">
                      {new Date(order.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <span className="num block text-[13px] text-ink">
                        {formatNGN(order.amount_ngn)}
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

      <p className="flex items-center gap-1.5 text-[12px] text-muted">
        <Download className="h-3.5 w-3.5" strokeWidth={1.6} />
        Export and date filters arrive with the accounting release.
      </p>
    </div>
  )
}
