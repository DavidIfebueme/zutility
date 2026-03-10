"use client"

import * as React from "react"
import Link from "next/link"
import { motion } from "motion/react"
import { CreditCard, ArrowRightLeft, Store, ArrowRight, Zap, Clock, CheckCircle2, AlertCircle, History } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { useAuthStore } from "@/store/auth"
import { useOrderStore } from "@/store/order"
import { useRate } from "@/lib/hooks/useRate"
import { formatNGN, formatZEC } from "@/lib/utils"

export default function DashboardPage() {
  const { user } = useAuthStore()
  const { activeOrder, status } = useOrderStore()
  const { rate } = useRate()

  // Mock recent transactions
  const recentTransactions = [
    { id: "ORD-1234", date: "2025-10-24T14:30:00Z", utility: "MTN Airtime", ngn: 5000, zec: "0.03333333", status: "completed" },
    { id: "ORD-1235", date: "2025-10-23T09:15:00Z", utility: "DSTV Premium", ngn: 24500, zec: "0.16333333", status: "completed" },
    { id: "ORD-1236", date: "2025-10-20T18:45:00Z", utility: "PHCN Prepaid", ngn: 10000, zec: "0.06666666", status: "failed" },
  ]

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="font-dela text-3xl tracking-tight">Dashboard</h1>
        <p className="text-text-secondary mt-2">
          Welcome back, {user?.displayName || user?.email || 'User'}.
        </p>
      </div>

      {/* First-visit banner / Active Order */}
      {activeOrder ? (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
        >
          <Card glow className="border-accent-zec/50 bg-bg-elevated overflow-hidden relative">
            <div className="absolute top-0 left-0 w-1 h-full bg-accent-zec" />
            <CardContent className="p-6 sm:p-8 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6">
              <div>
                <div className="flex items-center gap-2 mb-2">
                  <Badge variant="warning" className="animate-pulse">Active Order</Badge>
                  <span className="text-sm font-mono text-text-muted">{activeOrder.order_id}</span>
                </div>
                <h3 className="text-xl font-semibold mb-1">Waiting for ZEC Payment</h3>
                <p className="text-text-secondary">
                  Send exactly <span className="font-mono text-accent-zec">{formatZEC(activeOrder.zec_amount)}</span> ZEC to complete your order.
                </p>
              </div>
              <Link href={`/pay/${activeOrder.order_id}`}>
                <Button variant="primary" className="w-full sm:w-auto shrink-0">
                  View Order <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </Link>
            </CardContent>
          </Card>
        </motion.div>
      ) : (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
        >
          <Card className="border-border-subtle bg-bg-elevated overflow-hidden relative">
            <div className="absolute top-0 left-0 w-1 h-full bg-accent-green" />
            <CardContent className="p-6 sm:p-8 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6">
              <div>
                <h3 className="text-xl font-semibold mb-2">Pay your first bill with ZEC</h3>
                <p className="text-text-secondary">
                  Experience fast, private utility payments without KYC.
                </p>
              </div>
              <Link href="/pay">
                <Button variant="primary" className="w-full sm:w-auto shrink-0">
                  Pay a Utility <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </Link>
            </CardContent>
          </Card>
        </motion.div>
      )}

      {/* Quick Actions Row */}
      <div className="grid gap-4 sm:grid-cols-3">
        <Link href="/pay">
          <Card glow className="h-full hover:-translate-y-1 transition-transform duration-200 cursor-pointer">
            <CardContent className="p-6 flex flex-col items-center text-center gap-4">
              <div className="h-12 w-12 rounded-full bg-accent-zec/10 text-accent-zec flex items-center justify-center">
                <CreditCard className="h-6 w-6" />
              </div>
              <div>
                <h4 className="font-semibold">Pay Utilities</h4>
                <p className="text-xs text-text-secondary mt-1">Airtime, DSTV, Electricity</p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link href="/otc">
          <Card className="h-full hover:-translate-y-1 transition-transform duration-200 cursor-pointer opacity-80">
            <CardContent className="p-6 flex flex-col items-center text-center gap-4 relative">
              <div className="absolute top-4 right-4">
                <Badge variant="coming-soon" className="scale-75 origin-top-right">SOON</Badge>
              </div>
              <div className="h-12 w-12 rounded-full bg-text-muted/20 text-text-secondary flex items-center justify-center">
                <ArrowRightLeft className="h-6 w-6" />
              </div>
              <div>
                <h4 className="font-semibold text-text-secondary">OTC Swap</h4>
                <p className="text-xs text-text-muted mt-1">Direct ZEC to Naira</p>
              </div>
            </CardContent>
          </Card>
        </Link>

        <Link href="/p2p">
          <Card className="h-full hover:-translate-y-1 transition-transform duration-200 cursor-pointer opacity-80">
            <CardContent className="p-6 flex flex-col items-center text-center gap-4 relative">
              <div className="absolute top-4 right-4">
                <Badge variant="coming-soon" className="scale-75 origin-top-right">SOON</Badge>
              </div>
              <div className="h-12 w-12 rounded-full bg-text-muted/20 text-text-secondary flex items-center justify-center">
                <Store className="h-6 w-6" />
              </div>
              <div>
                <h4 className="font-semibold text-text-secondary">P2P Trade</h4>
                <p className="text-xs text-text-muted mt-1">Escrow marketplace</p>
              </div>
            </CardContent>
          </Card>
        </Link>
      </div>

      <div className="grid gap-8 lg:grid-cols-3">
        {/* Recent Transactions */}
        <div className="lg:col-span-2">
          <Card className="h-full">
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-xl">Recent Transactions</CardTitle>
              <Link href="/history">
                <Button variant="ghost" size="sm" className="text-text-secondary hover:text-text-primary">
                  View all
                </Button>
              </Link>
            </CardHeader>
            <CardContent>
              {recentTransactions.length > 0 ? (
                <div className="space-y-4 mt-4">
                  {recentTransactions.map((tx) => (
                    <div key={tx.id} className="flex items-center justify-between p-4 rounded-lg bg-bg-elevated border border-border-subtle">
                      <div className="flex items-center gap-4">
                        <div className="h-10 w-10 rounded-full bg-bg-surface flex items-center justify-center">
                          {tx.status === 'completed' ? (
                            <CheckCircle2 className="h-5 w-5 text-accent-green" />
                          ) : tx.status === 'failed' ? (
                            <AlertCircle className="h-5 w-5 text-accent-red" />
                          ) : (
                            <Clock className="h-5 w-5 text-accent-zec" />
                          )}
                        </div>
                        <div>
                          <p className="font-medium text-text-primary">{tx.utility}</p>
                          <p className="text-xs text-text-muted font-mono">{tx.id} • {new Date(tx.date).toLocaleDateString()}</p>
                        </div>
                      </div>
                      <div className="text-right">
                        <p className="font-medium text-text-primary">{formatNGN(tx.ngn)}</p>
                        <p className="text-xs text-text-secondary font-mono">{formatZEC(tx.zec)} ZEC</p>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center py-12 text-center">
                  <div className="h-16 w-16 rounded-full bg-bg-elevated flex items-center justify-center mb-4 text-text-muted">
                    <History className="h-8 w-8" />
                  </div>
                  <h4 className="text-lg font-medium mb-2">No transactions yet</h4>
                  <p className="text-text-secondary mb-6 max-w-sm">
                    Your recent utility payments and trades will appear here.
                  </p>
                  <Link href="/pay">
                    <Button variant="secondary">Make a Payment</Button>
                  </Link>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Rate Widget */}
        <div>
          <Card className="h-full bg-bg-elevated border-border-subtle">
            <CardHeader>
              <CardTitle className="text-xl">Market Rate</CardTitle>
              <CardDescription>ZEC to NGN</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col gap-6">
                <div>
                  <div className="text-3xl font-dela text-text-primary mb-2">
                    {rate ? formatNGN(rate.zec_ngn) : "₦---"}
                  </div>
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-accent-green flex items-center gap-1 bg-accent-green/10 px-2 py-0.5 rounded">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="22 7 13.5 15.5 8.5 10.5 2 17"></polyline>
                        <polyline points="16 7 22 7 22 13"></polyline>
                      </svg>
                      +2.4%
                    </span>
                    <span className="text-text-muted">Past 24h</span>
                  </div>
                </div>

                {/* Mock Sparkline */}
                <div className="h-24 w-full mt-4 relative">
                  <svg className="w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 40">
                    <path
                      d="M0,30 Q10,25 20,28 T40,20 T60,25 T80,10 T100,5"
                      fill="none"
                      stroke="var(--color-accent-zec)"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                    <path
                      d="M0,30 Q10,25 20,28 T40,20 T60,25 T80,10 T100,5 L100,40 L0,40 Z"
                      fill="url(#gradient)"
                      stroke="none"
                    />
                    <defs>
                      <linearGradient id="gradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0%" stopColor="var(--color-accent-zec)" stopOpacity="0.2" />
                        <stop offset="100%" stopColor="var(--color-accent-zec)" stopOpacity="0" />
                      </linearGradient>
                    </defs>
                  </svg>
                </div>
                
                <div className="text-xs text-text-muted text-center mt-2">
                  Last updated: {rate?.updated_at ? new Date(rate.updated_at).toLocaleTimeString() : '---'}
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
