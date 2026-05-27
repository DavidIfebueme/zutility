"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { motion } from "motion/react"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import ShieldCheck from "@/components/icons/shield-check"
import InfoCircleIcon from "@/components/icons/info-circle-icon"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import MagnifierIcon from "@/components/icons/magnifier-icon"
import CheckedIcon from "@/components/icons/checked-icon"
import { Loader2 } from "lucide-react"
import { getCategoryIcon } from "@/lib/category-icon-map"
import { getBrandIcon } from "@/lib/brand-icon-map"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { UTILITIES, UTILITY_CATEGORIES, type Utility, type UtilityType } from "@/lib/constants"
import { useOrderStore } from "@/store/order"
import { useRate } from "@/lib/hooks/useRate"
import { apiGet, apiPost } from "@/lib/api"
import { CreateOrderResponse, UtilityVariationResponse, UtilityValidateResponse } from "@/lib/types"
import { formatNGN } from "@/lib/utils"
import { useCurrency } from "@/lib/hooks/useCurrency"
import { formatCurrency, formatLocalAmount, convertFromNGN, convertToNGN, CURRENCIES, type FxRates } from "@/lib/currency"
import { toast } from "sonner"

const orderSchema = z.object({
  utilityId: z.string().min(1, "Please select a utility"),
  serviceRef: z.string().min(1, "Please enter a reference number"),
  amountNgn: z.number().min(100, "Minimum amount is 100").max(5000000, "Maximum amount is 50,000,000"),
  addressType: z.enum(["shielded", "transparent"]),
  variationCode: z.string().optional(),
})

type OrderFormValues = z.infer<typeof orderSchema>

function renderCategoryIcon(type: string, size = 16) {
  const Icon = getCategoryIcon(type as any)
  return <Icon size={size} />
}

function renderBrandIcon(brandIconId: string, size = 24) {
  const Icon = getBrandIcon(brandIconId as any)
  return <Icon size={size} />
}

export default function PayPage() {
  const router = useRouter()
  const { setActiveOrder } = useOrderStore()
  const { rate } = useRate()
  const currency = useCurrency()
  const currencySymbol = CURRENCIES[currency]?.symbol || '₦'
  const [isLoading, setIsLoading] = React.useState(false)
  const [activeCategory, setActiveCategory] = React.useState<UtilityType | 'school'>('airtime')
  const [variations, setVariations] = React.useState<UtilityVariationResponse[]>([])
  const [loadingVariations, setLoadingVariations] = React.useState(false)
  const [validating, setValidating] = React.useState(false)
  const [validated, setValidated] = React.useState<{ valid: boolean; customer_name: string | null } | null>(null)
  const [displayAmount, setDisplayAmount] = React.useState<string>("1000")

  const fxRates: FxRates | null = rate ? {
    usd_ngn: parseFloat(rate.usd_ngn) || 1,
    usd_kes: parseFloat(rate.usd_kes) || 0,
    usd_ghs: parseFloat(rate.usd_ghs) || 0,
    usd_zar: parseFloat(rate.usd_zar) || 0,
    usd_egp: parseFloat(rate.usd_egp) || 0,
  } : null

  const ngnFromDisplay = React.useCallback((localAmount: number): number => {
    if (!fxRates) return localAmount
    return convertToNGN(localAmount, currency, fxRates)
  }, [currency, fxRates])

  const localFromNgn = React.useCallback((ngnAmount: number): number => {
    if (!fxRates) return ngnAmount
    return convertFromNGN(ngnAmount, currency, fxRates)
  }, [currency, fxRates])

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
      variationCode: "",
    },
  })

  const selectedUtilityId = watch("utilityId")
  const amountNgn = watch("amountNgn")
  const addressType = watch("addressType")
  const variationCode = watch("variationCode")
  const serviceRef = watch("serviceRef")

  const selectedUtility = UTILITIES.find(u => u.id === selectedUtilityId)

  const filteredUtilities = UTILITIES.filter(u => u.type === activeCategory)

  const effectiveAmount = React.useMemo(() => {
    if (!selectedUtility) return amountNgn
    if (selectedUtility.fixedAmountKobo) return selectedUtility.fixedAmountKobo / 100
    if (selectedUtility.hasVariations && variationCode) {
      const variation = variations.find(v => v.variation_code === variationCode)
      if (variation?.amount) return variation.amount
    }
    return amountNgn
  }, [selectedUtility, amountNgn, variationCode, variations])

  const estimatedZec = React.useMemo(() => {
    if (!rate || !effectiveAmount) return "0.00000000"
    const zecNgn = parseFloat(rate.zec_ngn)
    if (zecNgn <= 0) return "0.00000000"
    return (effectiveAmount / zecNgn).toFixed(8)
  }, [rate, effectiveAmount])

  React.useEffect(() => {
    if (currency === 'NGN' || !fxRates) {
      setDisplayAmount(String(amountNgn))
    } else {
      setDisplayAmount(localFromNgn(amountNgn).toFixed(2))
    }
  }, [amountNgn, currency, fxRates, localFromNgn])

  const handleDisplayAmountChange = React.useCallback((raw: string) => {
    setDisplayAmount(raw)
    const parsed = parseFloat(raw)
    if (isNaN(parsed) || parsed <= 0) {
      setValue("amountNgn", 0, { shouldValidate: true })
      return
    }
    if (currency === 'NGN' || !fxRates) {
      setValue("amountNgn", Math.round(parsed), { shouldValidate: true })
    } else {
      const ngn = ngnFromDisplay(parsed)
      setValue("amountNgn", Math.round(ngn), { shouldValidate: true })
    }
  }, [currency, fxRates, ngnFromDisplay, setValue])

  React.useEffect(() => {
    setValidated(null)
    setVariations([])
    setValue("variationCode", "")
    if (selectedUtility?.fixedAmountKobo) {
      setValue("amountNgn", selectedUtility.fixedAmountKobo / 100)
    }
  }, [selectedUtilityId, selectedUtility, setValue])

  React.useEffect(() => {
    if (!selectedUtility?.hasVariations) {
      setVariations([])
      return
    }
    setLoadingVariations(true)
    apiGet<UtilityVariationResponse[]>(`/api/v1/utilities/${selectedUtility.slug}/variations`)
      .then(setVariations)
      .catch(() => setVariations([]))
      .finally(() => setLoadingVariations(false))
  }, [selectedUtility])

  const handleValidate = React.useCallback(async () => {
    if (!selectedUtility || !serviceRef || serviceRef.trim().length < 5) return
    setValidating(true)
    try {
      const result = await apiGet<UtilityValidateResponse>(
        `/api/v1/utilities/${selectedUtility.slug}/validate?ref=${encodeURIComponent(serviceRef)}`
      )
      setValidated(result)
      if (!result.valid) {
        toast.error("Invalid reference number")
      } else if (result.customer_name) {
        toast.success(`Verified: ${result.customer_name}`)
      } else {
        toast.success("Reference validated")
      }
    } catch {
      toast.error("Validation failed")
    } finally {
      setValidating(false)
    }
  }, [selectedUtility, serviceRef])

  const onSubmit = async (data: OrderFormValues) => {
    if (!selectedUtility) return

    setIsLoading(true)
    try {
      const utilityType = selectedUtility.type === 'school' ? 'school_fees' :
                          selectedUtility.type === 'tv' ? selectedUtility.slug :
                          selectedUtility.slug === 'waec-registration' || selectedUtility.slug === 'waec-result-checker' ? 'waec' :
                          selectedUtility.slug === 'jamb' ? 'jamb' :
                          selectedUtility.type
      const order = await apiPost<CreateOrderResponse>("/api/v1/orders/create", {
        utility_type: utilityType,
        utility_slug: selectedUtility.slug,
        service_ref: data.serviceRef,
        amount_ngn: Math.round(effectiveAmount),
        zec_address_type: data.addressType,
        variation_code: data.variationCode || undefined,
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

  return (
    <div className="max-w-3xl mx-auto space-y-8">
      <div>
        <h1 className="font-dela text-3xl tracking-tight">Pay Utilities</h1>
        <p className="text-text-secondary mt-2">
          Select a service and pay directly with Zcash. No KYC required.
        </p>
      </div>

      <form onSubmit={handleSubmit(onSubmit)} className="space-y-8">
        <Card className="border-border-subtle bg-bg-elevated">
          <CardHeader>
            <CardTitle className="text-xl flex items-center gap-2">
              <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent-zec text-bg-void text-sm font-bold">1</span>
              Select Service
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2 mb-6">
              {UTILITY_CATEGORIES.map((cat) => (
                <button
                  key={cat.type}
                  type="button"
                  onClick={() => setActiveCategory(cat.type)}
                  className={`px-4 py-2 rounded-full text-sm font-medium border transition-colors flex items-center gap-2 ${
                    activeCategory === cat.type
                      ? "border-accent-zec bg-accent-zec/10 text-accent-zec"
                      : "border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {renderCategoryIcon(cat.type, 16)}
                  {cat.label}
                </button>
              ))}
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {filteredUtilities.map((u) => {
                const isSelected = selectedUtilityId === u.id
                return (
                  <div
                    key={u.id}
                    onClick={() => setValue("utilityId", u.id, { shouldValidate: true })}
                    className={`cursor-pointer rounded-xl border p-4 flex flex-col items-center text-center gap-2 transition-all duration-200 ${
                      isSelected
                        ? "border-accent-zec bg-accent-zec/10 shadow-[0_0_15px_rgba(244,183,40,0.1)]"
                        : "border-border-subtle bg-bg-surface hover:border-text-muted"
                    }`}
                  >
                    <div className={`h-10 w-10 rounded-full flex items-center justify-center ${
                      isSelected ? "bg-accent-zec text-bg-void" : "bg-bg-elevated text-text-muted"
                    }`}>
                      {renderBrandIcon(u.brandIcon)}
                    </div>
                    <div>
                      <h4 className={`font-semibold text-sm ${isSelected ? "text-accent-zec" : "text-text-primary"}`}>
                        {u.name}
                      </h4>
                      <p className="text-xs text-text-muted mt-0.5">{u.descriptor}</p>
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
                <div className="flex items-center justify-between">
                  <label className="text-sm font-medium text-text-secondary">
                    {selectedUtility?.serviceRefLabel || 'Reference Number'}
                  </label>
                  {serviceRef && serviceRef.length >= 5 && (
                    <button
                      type="button"
                      onClick={handleValidate}
                      disabled={validating}
                      className="flex items-center gap-1.5 text-xs font-medium text-accent-zec hover:text-accent-zec/80 disabled:opacity-50"
                    >
                      {validating ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <MagnifierIcon size={14} />}
                      {validated ? "Re-validate" : "Validate"}
                    </button>
                  )}
                </div>
                <Input
                  {...register("serviceRef")}
                  placeholder={selectedUtility?.serviceRefPlaceholder || 'Enter number...'}
                  error={errors.serviceRef?.message}
                />
                {validated && (
                  <div className={`flex items-center gap-1.5 text-xs ${validated.valid ? 'text-accent-green' : 'text-accent-red'}`}>
                    {validated.valid ? <CheckedIcon size={14} /> : <InfoCircleIcon size={14} />}
                    {validated.customer_name ? `Verified: ${validated.customer_name}` : validated.valid ? 'Validated' : 'Invalid reference'}
                  </div>
                )}
              </div>

              {selectedUtility?.hasVariations && (
                <div className="space-y-2">
                  <label className="text-sm font-medium text-text-secondary">Select Plan</label>
                  {loadingVariations ? (
                    <div className="flex items-center gap-2 text-sm text-text-muted">
                      <Loader2 className="h-4 w-4 animate-spin" /> Loading plans...
                    </div>
                  ) : variations.length > 0 ? (
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 max-h-64 overflow-y-auto">
                      {variations.map((v) => {
                        const isSelected = variationCode === v.variation_code
                        return (
                          <div
                            key={v.variation_code}
                            onClick={() => {
                              setValue("variationCode", v.variation_code, { shouldValidate: true })
                              if (v.amount) setValue("amountNgn", v.amount, { shouldValidate: true })
                            }}
                            className={`cursor-pointer rounded-lg border p-3 transition-all ${
                              isSelected
                                ? "border-accent-zec bg-accent-zec/10"
                                : "border-border-subtle bg-bg-surface hover:border-text-muted"
                            }`}
                          >
                            <p className={`text-sm font-medium ${isSelected ? "text-accent-zec" : "text-text-primary"}`}>
                              {v.name}
                            </p>
                            {v.amount && (
                              <p className="text-xs text-text-muted mt-0.5">{fxRates ? formatLocalAmount(v.amount, currency, fxRates) : formatCurrency(v.amount, 'NGN')}</p>
                            )}
                          </div>
                        )
                      })}
                    </div>
                  ) : (
                    <p className="text-sm text-text-muted">No plans available</p>
                  )}
                </div>
              )}

              {selectedUtility?.hasAmountPicker && !selectedUtility.fixedAmountKobo && (
                <div className="space-y-2">
                  <label className="text-sm font-medium text-text-secondary">Amount</label>
                  <div className="relative">
                    <span className="absolute left-4 top-1/2 -translate-y-1/2 text-text-muted font-medium">{currencySymbol}</span>
                    <Input
                      type="number"
                      value={displayAmount}
                      onChange={(e) => handleDisplayAmountChange(e.target.value)}
                      className="pl-8"
                      error={errors.amountNgn?.message}
                    />
                  </div>
                  {selectedUtility.quickAmounts && (
                    <div className="flex flex-wrap gap-2 mt-3">
                      {selectedUtility.quickAmounts.map((amt) => (
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
                          {fxRates ? formatLocalAmount(amt, currency, fxRates) : formatCurrency(amt, 'NGN')}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {selectedUtility?.fixedAmountKobo && (
                <div className="rounded-lg bg-bg-surface border border-border-subtle p-4">
                  <p className="text-sm text-text-secondary">Fixed Amount</p>
                  <p className="text-2xl font-dela text-text-primary mt-1">
                    {fxRates ? formatLocalAmount(selectedUtility.fixedAmountKobo / 100, currency, fxRates) : formatCurrency(selectedUtility.fixedAmountKobo / 100, 'NGN')}
                  </p>
                </div>
              )}

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
                        <ShieldCheck size={20} className={addressType === "shielded" ? "text-accent-zec" : "text-text-muted"} />
                        <span className="font-medium">Shielded (z-address)</span>
                      </div>
                      <div className={`h-4 w-4 rounded-full border flex items-center justify-center ${
                        addressType === "shielded" ? "border-accent-zec" : "border-text-muted"
                      }`}>
                        {addressType === "shielded" && <div className="h-2 w-2 rounded-full bg-accent-zec" />}
                      </div>
                    </div>
                    <p className="text-xs text-text-secondary">Full privacy. Takes ~4 mins to confirm (3 confs).</p>
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

        <motion.div
          initial={false}
          animate={{ opacity: selectedUtilityId && effectiveAmount ? 1 : 0.5, pointerEvents: selectedUtilityId && effectiveAmount ? "auto" : "none" }}
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
                  {currency !== 'NGN' && effectiveAmount > 0 && (
                    <p className="text-xs text-text-muted mt-1">
                      ≈ {formatCurrency(effectiveAmount, 'NGN')}
                    </p>
                  )}
                  <div className="flex items-center gap-1.5 text-xs text-text-muted mt-1">
                    <InfoCircleIcon size={14} />
                    Rate locked for 15 minutes after creation
                  </div>
                </div>

                <Button
                  type="submit"
                  size="lg"
                  className="w-full sm:w-auto h-14 px-8 text-base"
                  loading={isLoading}
                  disabled={!selectedUtilityId || !effectiveAmount || !!errors.serviceRef}
                >
                  Create Order <ArrowNarrowRightIcon size={20} className="ml-2" />
                </Button>
              </div>
            </CardContent>
          </Card>
        </motion.div>
      </form>
    </div>
  )
}
