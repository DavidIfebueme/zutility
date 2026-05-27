"use client"

import * as React from "react"
import Link from "next/link"
import { motion } from "motion/react"
import ClockIcon from "@/components/icons/clock-icon"
import CheckedIcon from "@/components/icons/checked-icon"
import TriangleAlertIcon from "@/components/icons/triangle-alert-icon"
import HistoryCircleIcon from "@/components/icons/history-circle-icon"
import { Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { apiGet } from "@/lib/api"
import { formatNGN, formatZEC } from "@/lib/utils"

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

function statusBadge(status: string) {
  switch (status) {
    case "Completed":
      return <Badge variant="success">Completed</Badge>
    case "Failed":
    case "Cancelled":
      return <Badge variant="error">{status}</Badge>
    case "Pending":
      return <Badge variant="warning">Pending</Badge>
    default:
      return <Badge variant="outline">{status}</Badge>
  }
}

export default function HistoryPage() {
  const [orders, setOrders] = React.useState<OrderHistoryItem[]>([])
  const [loading, setLoading] = React.useState(true)

  React.useEffect(() => {
    apiGet<OrderHistoryItem[]>("/api/v1/orders/history")
      .then(setOrders)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  return (
    <div className="space-y-8">
      <div>
        <h1 className="font-dela text-3xl tracking-tight">Order History</h1>
        <p className="text-text-secondary mt-2">All your past utility payments.</p>
      </div>

      {loading ? (
        <div className="flex justify-center py-20">
          <Loader2 className="h-8 w-8 animate-spin text-text-muted" />
        </div>
      ) : orders.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="h-16 w-16 rounded-full bg-bg-elevated flex items-center justify-center mb-4 text-text-muted">
            <HistoryCircleIcon size={32} />
          </div>
          <h4 className="text-lg font-medium mb-2">No transactions yet</h4>
          <p className="text-text-secondary mb-6 max-w-sm">
            Your utility payment history will appear here.
          </p>
          <Link href="/pay">
            <Button variant="secondary">Make a Payment</Button>
          </Link>
        </div>
      ) : (
        <div className="space-y-3">
          {orders.map((order) => (
            <motion.div
              key={order.order_id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.2 }}
            >
              <Card className="hover:border-border-default transition-colors">
                <CardContent className="p-4 sm:p-6 flex items-center justify-between gap-4">
                  <div className="flex items-center gap-4 min-w-0">
                    <div className="h-10 w-10 rounded-full bg-bg-surface flex items-center justify-center shrink-0">
                      {order.status === "Completed" ? (
                        <CheckedIcon size={20} className="text-accent-green" />
                      ) : order.status === "Failed" || order.status === "Cancelled" ? (
                        <TriangleAlertIcon size={20} className="text-accent-red" />
                      ) : (
                        <ClockIcon size={20} className="text-accent-zec" />
                      )}
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <p className="font-medium text-text-primary truncate">{order.utility_slug}</p>
                        {statusBadge(order.status)}
                      </div>
                      <p className="text-xs text-text-muted font-mono">
                        {order.order_id.slice(0, 8)} • {new Date(order.created_at).toLocaleDateString()} {new Date(order.created_at).toLocaleTimeString()}
                      </p>
                    </div>
                  </div>
                  <div className="text-right shrink-0">
                    <p className="font-medium text-text-primary">{formatNGN(order.amount_ngn)}</p>
                    <p className="text-xs text-text-secondary font-mono">{formatZEC(order.zec_amount)} ZEC</p>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  )
}
