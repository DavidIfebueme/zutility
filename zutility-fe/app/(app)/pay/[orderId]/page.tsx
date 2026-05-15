"use client"

import * as React from "react"
import { useParams, useRouter } from "next/navigation"
import { motion, AnimatePresence } from "motion/react"
import { QRCodeSVG } from "qrcode.react"
import { Copy, CheckCircle2, AlertCircle, Clock, ArrowLeft, Zap, XCircle, TimerOff } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { CopyField } from "@/components/ui/copy-field"
import { Stepper } from "@/components/ui/stepper"
import { CountdownTimer } from "@/components/ui/countdown-timer"
import { ConfirmationProgress } from "@/components/ui/confirmation-progress"
import { useOrderStore } from "@/store/order"
import { useOrderStream } from "@/lib/hooks/useOrderStream"
import { apiGet } from "@/lib/api"
import { formatZEC } from "@/lib/utils"
import { UTILITIES } from "@/lib/constants"
import type { CreateOrderResponse } from "@/lib/types"

function getCompletionInfo(slug: string): { title: string; description: string; tokenLabel: string } {
  const u = UTILITIES.find(u => u.slug === slug)
  if (!u) return { title: 'Order Complete!', description: 'Your utility has been successfully delivered.', tokenLabel: 'Token / PIN' }
  
  switch (u.type) {
    case 'airtime':
      return { title: 'Airtime Delivered!', description: 'Your phone has been topped up successfully.', tokenLabel: 'Reference' }
    case 'data':
      return { title: 'Data Bundle Activated!', description: 'Your data bundle has been activated on the number provided.', tokenLabel: 'Reference' }
    case 'tv':
      return { title: 'TV Subscription Renewed!', description: 'Your TV subscription has been renewed successfully.', tokenLabel: 'Reference' }
    case 'electricity':
      return { title: 'Electricity Token Delivered!', description: 'Your prepaid electricity token is ready. Enter it on your meter to load units.', tokenLabel: 'Meter Token' }
    case 'education':
      return { title: 'PIN Delivered!', description: u.name.includes('JAMB') ? 'Your JAMB registration PIN has been delivered.' : 'Your exam PIN has been delivered. Use it on the official portal.', tokenLabel: 'PIN' }
    case 'school':
      return { title: 'School Fees Paid!', description: 'Your school fees payment has been confirmed.', tokenLabel: 'Payment Reference' }
    default:
      return { title: 'Order Complete!', description: 'Your utility has been successfully delivered.', tokenLabel: 'Token / PIN' }
  }
}

export default function OrderPage() {
  const params = useParams()
  const router = useRouter()
  const orderId = params.orderId as string
  
  const { activeOrder, setActiveOrder, clearActiveOrder } = useOrderStore()
  const [recoveredOrder, setRecoveredOrder] = React.useState<CreateOrderResponse | null>(null)
  const [recovering, setRecovering] = React.useState(false)
  
  const isCurrentOrder = activeOrder?.order_id === orderId
  const order = isCurrentOrder ? activeOrder : recoveredOrder

  React.useEffect(() => {
    if (!isCurrentOrder && !recoveredOrder && !recovering) {
      const token = new URLSearchParams(window.location.search).get('token')
      if (token && orderId) {
        setRecovering(true)
        apiGet<CreateOrderResponse>(`/api/v1/orders/${orderId}?token=${encodeURIComponent(token)}`)
          .then((data) => {
            setRecoveredOrder(data)
          })
          .catch(() => {
            toast.error("Could not load order")
          })
          .finally(() => setRecovering(false))
      }
    }
  }, [isCurrentOrder, recoveredOrder, recovering, orderId])

  const { status, confirmations, latestEvent, isConnected } = useOrderStream(
    order ? orderId : null,
    order ? order.order_access_token : null
  )

  if (!order) {
    return (
      <div className="flex flex-col items-center justify-center py-24 text-center">
        <AlertCircle className="h-12 w-12 text-accent-red mb-4" />
        <h2 className="text-2xl font-dela mb-2">Order Not Found</h2>
        <p className="text-text-secondary mb-6">
          {recovering ? 'Loading order...' : "This order doesn't exist or you don't have access to it."}
        </p>
        <Button onClick={() => router.push('/dashboard')}>Return to Dashboard</Button>
      </div>
    )
  }

  const steps = [
    { label: "Awaiting Payment" },
    { label: "Confirming" },
    { label: "Dispatching" },
    { label: "Completed" }
  ]

  const getStepIndex = () => {
    switch (status) {
      case 'awaiting_payment': return 0
      case 'payment_detected': return 1
      case 'payment_confirmed': return 2
      case 'utility_dispatching': return 2
      case 'completed': return 4
      case 'expired': return 0
      case 'failed': return 2
      default: return 0
    }
  }

  const handleExpire = () => {
    if (status === 'awaiting_payment') {
      toast.error("Order expired")
    }
  }

  const addressType = order.required_confirmations > 3 ? 'shielded' : 'transparent'

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => router.push('/dashboard')}>
          <ArrowLeft className="h-5 w-5" />
        </Button>
        <div>
          <h1 className="font-dela text-2xl tracking-tight">Order {orderId.slice(0, 8)}...</h1>
          <p className="text-text-secondary text-sm">
            {addressType === 'shielded' ? 'Shielded' : 'Transparent'} Payment
            {!isConnected && status !== 'completed' && status !== 'expired' && status !== 'failed' && (
              <span className="ml-2 text-accent-amber">&#x2022; Reconnecting...</span>
            )}
          </p>
        </div>
        <div className="ml-auto">
          <Badge variant={
            status === 'completed' ? 'success' :
            status === 'failed' || status === 'expired' ? 'error' :
            'warning'
          }>
            {status.replace(/_/g, ' ').toUpperCase()}
          </Badge>
        </div>
      </div>

      <Card className="border-border-subtle bg-bg-elevated">
        <CardContent className="p-6 sm:p-8">
          <Stepper steps={steps} currentStep={getStepIndex()} className="mb-8" />
          
          <AnimatePresence mode="wait">
            {status === 'awaiting_payment' && (
              <motion.div
                key="awaiting"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                className="grid md:grid-cols-2 gap-8"
              >
                <div className="flex flex-col items-center justify-center p-6 border border-border-subtle rounded-xl bg-bg-surface relative overflow-hidden">
                  <div className="absolute top-0 left-0 w-full h-1 bg-accent-zec" />
                  <div className="bg-white p-4 rounded-xl mb-6">
                    <QRCodeSVG
                      value={order.qr_data}
                      size={200}
                      level="H"
                      includeMargin={false}
                    />
                  </div>
                  <div className="flex items-center gap-2 text-sm text-text-secondary mb-2">
                    <Clock className="h-4 w-4" />
                    Expires in: <CountdownTimer expiresAt={order.expires_at} onExpire={handleExpire} />
                  </div>
                  <p className="text-xs text-text-muted text-center max-w-[200px]">
                    Scan with Zashi, Nighthawk, Zingo, or any Zcash wallet
                  </p>
                </div>

                <div className="space-y-6">
                  <div>
                    <h3 className="text-lg font-semibold mb-4">Payment Details</h3>
                    <div className="space-y-4">
                      <CopyField label="Amount to send" value={formatZEC(order.zec_amount)} />
                      <CopyField label="Deposit Address" value={order.deposit_address} />
                    </div>
                  </div>

                  <div className="rounded-lg bg-accent-zec/10 p-4 border border-accent-zec/20">
                    <div className="flex items-start gap-3">
                      <AlertCircle className="h-5 w-5 text-accent-zec shrink-0 mt-0.5" />
                      <div className="text-sm text-text-primary">
                        <p className="font-medium text-accent-zec mb-1">Important</p>
                        <ul className="list-disc pl-4 space-y-1 text-text-secondary">
                          <li>Send exactly <span className="font-mono text-text-primary">{formatZEC(order.zec_amount)}</span> ZEC</li>
                          <li>Do not include transaction fees in this amount</li>
                          <li>Takes ~4 mins to confirm ({order.required_confirmations} confirmations)</li>
                        </ul>
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>
            )}

            {(status === 'payment_detected' || status === 'payment_confirmed') && (
              <motion.div
                key="confirming"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-8"
              >
                <ConfirmationProgress
                  current={confirmations}
                  required={order.required_confirmations}
                  addressType={addressType}
                />
                <h3 className="text-xl font-semibold mt-6 mb-2">
                  {status === 'payment_confirmed' ? 'Payment Confirmed!' : 'Confirming Payment...'}
                </h3>
                <p className="text-text-secondary text-center max-w-md">
                  {status === 'payment_confirmed' 
                    ? 'Your payment has been fully confirmed on the Zcash network. We are now dispatching your utility.'
                    : `We've detected your payment on the network. Waiting for ${order.required_confirmations} confirmations to ensure finality.`}
                </p>
                <p className="text-sm text-text-muted mt-2">
                  {confirmations} / {order.required_confirmations} confirmations
                </p>
              </motion.div>
            )}

            {status === 'utility_dispatching' && (
              <motion.div
                key="dispatching"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-16"
              >
                <div className="relative flex h-24 w-24 items-center justify-center rounded-full bg-accent-zec/10 mb-6">
                  <div className="absolute inset-0 rounded-full border-4 border-accent-zec border-t-transparent animate-spin" />
                  <Zap className="h-10 w-10 text-accent-zec animate-pulse" />
                </div>
                <h3 className="text-2xl font-dela mb-2">Dispatching Utility</h3>
                <p className="text-text-secondary text-center max-w-md">
                  Connecting to the provider to deliver your service. This usually takes less than a minute.
                </p>
              </motion.div>
            )}

            {status === 'completed' && (() => {
                const info = getCompletionInfo(order.utility_slug || '')
                return (
              <motion.div
                key="completed"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-12"
              >
                <div className="flex h-24 w-24 items-center justify-center rounded-full bg-accent-green/10 mb-6">
                  <CheckCircle2 className="h-12 w-12 text-accent-green" />
                </div>
                <h3 className="text-3xl font-dela text-accent-green mb-2">{info.title}</h3>
                <p className="text-text-secondary text-center max-w-md mb-8">
                  {info.description}
                </p>

                {latestEvent?.event === 'completed' && latestEvent.delivery_token && (
                  <div className="w-full max-w-md bg-bg-surface border border-border-subtle rounded-xl p-6 text-center">
                    <p className="text-sm text-text-secondary uppercase tracking-wider mb-2">{info.tokenLabel}</p>
                    <p className="text-3xl font-mono font-bold tracking-widest text-text-primary mb-4">
                      {latestEvent.delivery_token}
                    </p>
                    <CopyField value={latestEvent.delivery_token} className="text-left" />
                  </div>
                )}

                <div className="mt-8 flex gap-4">
                  <Button variant="secondary" onClick={() => router.push('/dashboard')}>
                    Back to Dashboard
                  </Button>
                  <Button variant="primary" onClick={() => {
                    clearActiveOrder()
                    router.push('/pay')
                  }}>
                    Make Another Payment
                  </Button>
                </div>
              </motion.div>
                )
            })()}

            {status === 'expired' && (
              <motion.div
                key="expired"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-12"
              >
                <div className="flex h-24 w-24 items-center justify-center rounded-full bg-accent-amber/10 mb-6">
                  <TimerOff className="h-12 w-12 text-accent-amber" />
                </div>
                <h3 className="text-2xl font-dela text-accent-amber mb-2">Order Expired</h3>
                <p className="text-text-secondary text-center max-w-md mb-8">
                  This order has expired because no payment was received in time. Please create a new order to try again.
                </p>
                <div className="flex gap-4">
                  <Button variant="secondary" onClick={() => router.push('/dashboard')}>
                    Back to Dashboard
                  </Button>
                  <Button variant="primary" onClick={() => {
                    clearActiveOrder()
                    router.push('/pay')
                  }}>
                    Create New Order
                  </Button>
                </div>
              </motion.div>
            )}

            {status === 'failed' && (
              <motion.div
                key="failed"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-12"
              >
                <div className="flex h-24 w-24 items-center justify-center rounded-full bg-accent-red/10 mb-6">
                  <XCircle className="h-12 w-12 text-accent-red" />
                </div>
                <h3 className="text-2xl font-dela text-accent-red mb-2">Order Failed</h3>
                <p className="text-text-secondary text-center max-w-md mb-8">
                  We were unable to process your utility payment. If you were charged, please contact support with your order ID.
                </p>
                <div className="flex gap-4">
                  <Button variant="secondary" onClick={() => router.push('/dashboard')}>
                    Back to Dashboard
                  </Button>
                  <Button variant="primary" onClick={() => {
                    clearActiveOrder()
                    router.push('/pay')
                  }}>
                    Try Again
                  </Button>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </CardContent>
      </Card>
    </div>
  )
}
