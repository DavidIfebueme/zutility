"use client"

import * as React from "react"
import { useParams, useRouter } from "next/navigation"
import { motion, AnimatePresence } from "motion/react"
import { QRCodeSVG } from "qrcode.react"
import { Copy, CheckCircle2, AlertCircle, Clock, ArrowLeft, ExternalLink, Zap } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { CopyField } from "@/components/ui/copy-field"
import { Stepper } from "@/components/ui/stepper"
import { CountdownTimer } from "@/components/ui/countdown-timer"
import { ConfirmationProgress } from "@/components/ui/confirmation-progress"
import { useOrderStore } from "@/store/order"
import { useOrderStream } from "@/lib/hooks/useOrderStream"
import { formatNGN, formatZEC } from "@/lib/utils"

export default function OrderPage() {
  const params = useParams()
  const router = useRouter()
  const orderId = params.orderId as string
  
  const { activeOrder, clearActiveOrder } = useOrderStore()
  
  // If no active order matches this ID, we'd normally fetch it.
  // For this demo, if it doesn't match, we redirect or show error.
  const isCurrentOrder = activeOrder?.order_id === orderId
  
  const { status, confirmations, latestEvent, isConnected } = useOrderStream(
    isCurrentOrder ? orderId : null,
    isCurrentOrder ? activeOrder.order_access_token : null
  )

  if (!isCurrentOrder) {
    return (
      <div className="flex flex-col items-center justify-center py-24 text-center">
        <AlertCircle className="h-12 w-12 text-accent-red mb-4" />
        <h2 className="text-2xl font-dela mb-2">Order Not Found</h2>
        <p className="text-text-secondary mb-6">This order doesn&apos;t exist or you don&apos;t have access to it.</p>
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
      case 'completed': return 4 // All complete
      default: return 0
    }
  }

  const handleExpire = () => {
    if (status === 'awaiting_payment') {
      // In a real app, the server would send an 'expired' event
      // We just mock it here if the timer runs out
      toast.error("Order expired")
      clearActiveOrder()
      router.push('/dashboard')
    }
  }

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => router.push('/dashboard')}>
          <ArrowLeft className="h-5 w-5" />
        </Button>
        <div>
          <h1 className="font-dela text-2xl tracking-tight">Order {orderId}</h1>
          <p className="text-text-secondary text-sm">
            {activeOrder.required_confirmations === 10 ? 'Shielded' : 'Transparent'} Payment
          </p>
        </div>
        <div className="ml-auto">
          <Badge variant={
            status === 'completed' ? 'success' :
            status === 'failed' || status === 'expired' ? 'error' :
            'warning'
          }>
            {status.replace('_', ' ').toUpperCase()}
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
                      value={activeOrder.qr_data}
                      size={200}
                      level="H"
                      includeMargin={false}
                    />
                  </div>
                  <div className="flex items-center gap-2 text-sm text-text-secondary mb-2">
                    <Clock className="h-4 w-4" />
                    Expires in: <CountdownTimer expiresAt={activeOrder.expires_at} onExpire={handleExpire} />
                  </div>
                  <p className="text-xs text-text-muted text-center max-w-[200px]">
                    Scan with Zashi, Nighthawk, or any Zcash wallet
                  </p>
                </div>

                <div className="space-y-6">
                  <div>
                    <h3 className="text-lg font-semibold mb-4">Payment Details</h3>
                    <div className="space-y-4">
                      <CopyField label="Amount to send" value={formatZEC(activeOrder.zec_amount)} />
                      <CopyField label="Deposit Address" value={activeOrder.deposit_address} />
                    </div>
                  </div>

                  <div className="rounded-lg bg-accent-zec/10 p-4 border border-accent-zec/20">
                    <div className="flex items-start gap-3">
                      <AlertCircle className="h-5 w-5 text-accent-zec shrink-0 mt-0.5" />
                      <div className="text-sm text-text-primary">
                        <p className="font-medium text-accent-zec mb-1">Important</p>
                        <ul className="list-disc pl-4 space-y-1 text-text-secondary">
                          <li>Send exactly <span className="font-mono text-text-primary">{formatZEC(activeOrder.zec_amount)}</span> ZEC</li>
                          <li>Do not include transaction fees in this amount</li>
                          <li>Send from a {activeOrder.required_confirmations === 10 ? 'shielded (z)' : 'transparent (t)'} address if possible</li>
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
                  required={activeOrder.required_confirmations}
                  addressType={activeOrder.required_confirmations === 10 ? 'shielded' : 'transparent'}
                />
                <h3 className="text-xl font-semibold mt-6 mb-2">
                  {status === 'payment_confirmed' ? 'Payment Confirmed!' : 'Confirming Payment...'}
                </h3>
                <p className="text-text-secondary text-center max-w-md">
                  {status === 'payment_confirmed' 
                    ? 'Your payment has been fully confirmed on the Zcash network. We are now dispatching your utility.'
                    : `We've detected your payment on the network. Waiting for ${activeOrder.required_confirmations} confirmations to ensure finality.`}
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

            {status === 'completed' && (
              <motion.div
                key="completed"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                className="flex flex-col items-center justify-center py-12"
              >
                <div className="flex h-24 w-24 items-center justify-center rounded-full bg-accent-green/10 mb-6">
                  <CheckCircle2 className="h-12 w-12 text-accent-green" />
                </div>
                <h3 className="text-3xl font-dela text-accent-green mb-2">Order Complete!</h3>
                <p className="text-text-secondary text-center max-w-md mb-8">
                  Your utility has been successfully delivered.
                </p>

                {latestEvent?.event === 'completed' && latestEvent.delivery_token && (
                  <div className="w-full max-w-md bg-bg-surface border border-border-subtle rounded-xl p-6 text-center">
                    <p className="text-sm text-text-secondary uppercase tracking-wider mb-2">Token / PIN</p>
                    <p className="text-3xl font-mono font-bold tracking-widest text-text-primary mb-4">
                      {latestEvent.delivery_token}
                    </p>
                    <CopyField value={latestEvent.delivery_token} className="text-left" />
                  </div>
                )}

                <div className="mt-8 flex gap-4">
                  <Button variant="outline" onClick={() => router.push('/dashboard')}>
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
            )}
          </AnimatePresence>
        </CardContent>
      </Card>
    </div>
  )
}
