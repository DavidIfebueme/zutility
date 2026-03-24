"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { motion } from "motion/react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Shield, Smartphone, Tv, Zap, ArrowRight, Info } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { UTILITIES } from "@/lib/constants"
import { useOrderStore } from "@/store/order"
import { useRate } from "@/lib/hooks/useRate"
import { apiPost } from "@/lib/api"
import { CreateOrderResponse } from "@/lib/types"
import { formatNGN } from "@/lib/utils"
import { toast } from "sonner"

const orderSchema = z.object({
  utilityId: z.string().min(1, "Please select a utility"),
  serviceRef: z.string().min(5, "Please enter a valid phone number or meter/smartcard number"),
  amountNgn: z.number().min(100, "Minimum amount is ₦100").max(100000, "Maximum amount is ₦100,000"),
  addressType: z.enum(["shielded", "transparent"]),
})

type OrderFormValues = z.infer<typeof orderSchema>

export default function PayPage() {
  const router = useRouter()
  const { setActiveOrder } = useOrderStore()
  const { rate } = useRate()
  const [isLoading, setIsLoading] = React.useState(false)

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<OrderFormValues>({
    resolver: zodResolver(orderSchema),
    defaultValues: {
      addressType: "shielded",
      amountNgn: 1000,
    },
  })

  const selectedUtilityId = watch("utilityId")
  const amountNgn = watch("amountNgn")
  const addressType = watch("addressType")

  const selectedUtility = UTILITIES.find(u => u.id === selectedUtilityId)

  // Calculate estimated ZEC
  const estimatedZec = React.useMemo(() => {
    if (!rate || !amountNgn) return "0.00000000"
    const zecNgn = parseFloat(rate.zec_ngn)
    if (zecNgn <= 0) return "0.00000000"
    return (amountNgn / zecNgn).toFixed(8)
  }, [rate, amountNgn])

  const onSubmit = async (data: OrderFormValues) => {
    if (!selectedUtility) return
    
    setIsLoading(true)
    try {
      const order = await apiPost<CreateOrderResponse>("/api/v1/orders/create", {
        utility_type: selectedUtility.type,
        utility_slug: selectedUtility.slug,
        service_ref: data.serviceRef,
        amount_ngn: data.amountNgn,
        zec_address_type: data.addressType,
      })

      setActiveOrder(order)
      toast.success("Order created successfully")
      router.push(`/pay/${order.order_id}`)
    } catch {
      toast.error("Failed to create order. Please try again.")
    } finally {
      setIsLoading(false)
    }
  }

  const getIcon = (type: string) => {
    switch (type) {
      case 'airtime': return <Smartphone className="h-6 w-6" />
      case 'tv': return <Tv className="h-6 w-6" />
      case 'electricity': return <Zap className="h-6 w-6" />
      default: return <Zap className="h-6 w-6" />
    }
  }

  return (
    <div className="max-w-3xl mx-auto space-y-8">
      <div>
        <h1 className="font-dela text-3xl tracking-tight">Pay Utilities</h1>
        <p className="text-text-secondary mt-2">
          Select a service and pay directly with Zcash. No KYC required.
        </p>
      </div>

      <form onSubmit={handleSubmit(onSubmit)} className="space-y-8">
        {/* Step 1: Select Utility */}
        <Card className="border-border-subtle bg-bg-elevated">
          <CardHeader>
            <CardTitle className="text-xl flex items-center gap-2">
              <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent-zec text-bg-void text-sm font-bold">1</span>
              Select Service
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {UTILITIES.map((u) => {
                const isSelected = selectedUtilityId === u.id
                return (
                  <div
                    key={u.id}
                    onClick={() => setValue("utilityId", u.id, { shouldValidate: true })}
                    className={`cursor-pointer rounded-xl border p-4 flex flex-col items-center text-center gap-3 transition-all duration-200 ${
                      isSelected 
                        ? "border-accent-zec bg-accent-zec/10 shadow-[0_0_15px_rgba(244,183,40,0.1)]" 
                        : "border-border-subtle bg-bg-surface hover:border-text-muted"
                    }`}
                  >
                    <div className={`h-12 w-12 rounded-full flex items-center justify-center ${
                      isSelected ? "bg-accent-zec text-bg-void" : "bg-bg-elevated text-text-muted"
                    }`}>
                      {getIcon(u.type)}
                    </div>
                    <div>
                      <h4 className={`font-semibold text-sm ${isSelected ? "text-accent-zec" : "text-text-primary"}`}>
                        {u.name}
                      </h4>
                    </div>
                  </div>
                )
              })}
            </div>
            {errors.utilityId && (
              <p className="mt-3 text-sm text-accent-red">{errors.utilityId.message}</p>
            )}
          </CardContent>
        </Card>

        {/* Step 2: Details */}
        <motion.div
          initial={false}
          animate={{ opacity: selectedUtilityId ? 1 : 0.5, pointerEvents: selectedUtilityId ? "auto" : "none" }}
        >
          <Card className="border-border-subtle bg-bg-elevated">
            <CardHeader>
              <CardTitle className="text-xl flex items-center gap-2">
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent-zec text-bg-void text-sm font-bold">2</span>
                Payment Details
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">
                  {selectedUtility?.type === 'airtime' ? 'Phone Number' : 
                   selectedUtility?.type === 'tv' ? 'Smartcard Number' : 
                   'Meter Number'}
                </label>
                <Input
                  {...register("serviceRef")}
                  placeholder={selectedUtility?.type === 'airtime' ? '080...' : 'Enter number...'}
                  error={errors.serviceRef?.message}
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-text-secondary">Amount (NGN)</label>
                <div className="relative">
                  <span className="absolute left-4 top-1/2 -translate-y-1/2 text-text-muted font-medium">₦</span>
                  <Input
                    type="number"
                    {...register("amountNgn", { valueAsNumber: true })}
                    className="pl-8"
                    error={errors.amountNgn?.message}
                  />
                </div>
                
                {/* Quick amount selectors */}
                <div className="flex flex-wrap gap-2 mt-3">
                  {[1000, 2000, 5000, 10000].map((amt) => (
                    <button
                      key={amt}
                      type="button"
                      onClick={() => setValue("amountNgn", amt, { shouldValidate: true })}
                      className={`px-3 py-1.5 rounded-full text-xs font-medium border transition-colors ${
                        amountNgn === amt 
                          ? "border-accent-zec bg-accent-zec/10 text-accent-zec" 
                          : "border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary"
                      }`}
                    >
                      {formatNGN(amt)}
                    </button>
                  ))}
                </div>
              </div>

              <div className="space-y-3 pt-4 border-t border-border-subtle">
                <label className="text-sm font-medium text-text-secondary">Zcash Address Type</label>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div
                    onClick={() => setValue("addressType", "shielded")}
                    className={`cursor-pointer rounded-lg border p-4 transition-all duration-200 ${
                      addressType === "shielded"
                        ? "border-accent-zec bg-accent-zec/5"
                        : "border-border-subtle bg-bg-surface hover:border-text-muted"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <Shield className={`h-5 w-5 ${addressType === "shielded" ? "text-accent-zec" : "text-text-muted"}`} />
                        <span className="font-medium">Shielded (z-address)</span>
                      </div>
                      <div className={`h-4 w-4 rounded-full border flex items-center justify-center ${
                        addressType === "shielded" ? "border-accent-zec" : "border-text-muted"
                      }`}>
                        {addressType === "shielded" && <div className="h-2 w-2 rounded-full bg-accent-zec" />}
                      </div>
                    </div>
                    <p className="text-xs text-text-secondary">Full privacy. Takes ~13 mins to confirm (10 confs).</p>
                  </div>

                  <div
                    onClick={() => setValue("addressType", "transparent")}
                    className={`cursor-pointer rounded-lg border p-4 transition-all duration-200 ${
                      addressType === "transparent"
                        ? "border-accent-zec bg-accent-zec/5"
                        : "border-border-subtle bg-bg-surface hover:border-text-muted"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <div className={`h-5 w-5 rounded-full border-2 border-dashed ${addressType === "transparent" ? "border-accent-zec" : "border-text-muted"}`} />
                        <span className="font-medium">Transparent (t-address)</span>
                      </div>
                      <div className={`h-4 w-4 rounded-full border flex items-center justify-center ${
                        addressType === "transparent" ? "border-accent-zec" : "border-text-muted"
                      }`}>
                        {addressType === "transparent" && <div className="h-2 w-2 rounded-full bg-accent-zec" />}
                      </div>
                    </div>
                    <p className="text-xs text-text-secondary">Public transaction. Takes ~4 mins to confirm (3 confs).</p>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </motion.div>

        {/* Summary & Submit */}
        <motion.div
          initial={false}
          animate={{ opacity: selectedUtilityId && amountNgn ? 1 : 0.5, pointerEvents: selectedUtilityId && amountNgn ? "auto" : "none" }}
        >
          <Card className="border-accent-zec/30 bg-bg-elevated">
            <CardContent className="p-6">
              <div className="flex flex-col sm:flex-row items-center justify-between gap-6">
                <div className="space-y-1 w-full sm:w-auto">
                  <p className="text-sm text-text-secondary">You will pay</p>
                  <div className="flex items-baseline gap-2">
                    <span className="text-3xl font-dela text-accent-zec">{estimatedZec}</span>
                    <span className="text-text-muted font-medium">ZEC</span>
                  </div>
                  <div className="flex items-center gap-1.5 text-xs text-text-muted mt-1">
                    <Info className="h-3.5 w-3.5" />
                    Rate locked for 15 minutes after creation
                  </div>
                </div>

                <Button 
                  type="submit" 
                  size="lg" 
                  className="w-full sm:w-auto h-14 px-8 text-base"
                  loading={isLoading}
                  disabled={!selectedUtilityId || !amountNgn || !!errors.amountNgn || !!errors.serviceRef}
                >
                  Create Order <ArrowRight className="ml-2 h-5 w-5" />
                </Button>
              </div>
            </CardContent>
          </Card>
        </motion.div>
      </form>
    </div>
  )
}
