"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { Check, Gift, Info, Loader2, Search, ShieldCheck } from "lucide-react"
import { BrandMark } from "@/components/ui/brand-mark"
import { BRAND_TILE_BACKGROUND } from "@/lib/brand-logo"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { UTILITIES, UTILITY_CATEGORIES, type UtilityType } from "@/lib/constants"
import { useOrderStore } from "@/store/order"
import { useAuthStore } from "@/store/auth"
import { useRate } from "@/lib/hooks/useRate"
import { apiGet, apiPost } from "@/lib/api"
import { CreateGiftResponse, CreateOrderResponse, UtilityVariationResponse, UtilityValidateResponse } from "@/lib/types"
import { useCurrency } from "@/lib/hooks/useCurrency"
import {
  CURRENCIES,
  convertFromNGN,
  convertToNGN,
  formatCurrency,
  formatLocalAmount,
  type FxRates,
} from "@/lib/currency"
import { toast } from "sonner"
import { cn } from "@/lib/utils"

const orderSchema = z.object({
  utilityId: z.string().min(1, "Select a service"),
  serviceRef: z.string(),
  amountNgn: z.number().min(100, "Minimum is 100").max(5000000, "Maximum is 5,000,000"),
  addressType: z.enum(["shielded", "transparent"]),
  variationCode: z.string().optional(),
})

const GIFT_MAX_NGN = 100_000
const GIFT_WINDOWS = [
  { hours: 12, label: "12 hours" },
  { hours: 24, label: "24 hours" },
  { hours: 72, label: "3 days" },
  { hours: 168, label: "7 days" },
]

type OrderFormValues = z.infer<typeof orderSchema>

function BrandTile({ brandIcon, selected }: { brandIcon: string; selected: boolean }) {
  const id = brandIcon as never
  return (
    <span
      className={cn(
        "flex h-10 w-10 shrink-0 items-center justify-center rounded-md",
        selected ? "bg-paper-3" : "bg-paper-2"
      )}
      style={BRAND_TILE_BACKGROUND[id] ? { backgroundColor: BRAND_TILE_BACKGROUND[id] } : undefined}
    >
      <BrandMark id={id} size={26} />
    </span>
  )
}

export default function PayPage() {
  const router = useRouter()
  const { setActiveOrder } = useOrderStore()
  const { user } = useAuthStore()
  const { rate } = useRate()
  const currency = useCurrency()
  const currencySymbol = CURRENCIES[currency]?.symbol || "₦"
  const [isLoading, setIsLoading] = React.useState(false)
  const [activeCategory, setActiveCategory] = React.useState<UtilityType | "school">("airtime")
  const [query, setQuery] = React.useState("")
  const [variations, setVariations] = React.useState<UtilityVariationResponse[]>([])
  const [loadingVariations, setLoadingVariations] = React.useState(false)
  const [validating, setValidating] = React.useState(false)
  const [validated, setValidated] = React.useState<UtilityValidateResponse | null>(null)
  const [validatedReference, setValidatedReference] = React.useState<string | null>(null)
  const [displayAmount, setDisplayAmount] = React.useState<string>("1000")
  const [isGift, setIsGift] = React.useState(false)
  const [giftMessage, setGiftMessage] = React.useState("")
  const [giftFallbackRef, setGiftFallbackRef] = React.useState("")
  const [giftWindowHours, setGiftWindowHours] = React.useState(168)
  const [giftHideName, setGiftHideName] = React.useState(false)

  const fxRates: FxRates | null = React.useMemo(
    () =>
      rate
        ? {
            usd_ngn: parseFloat(rate.usd_ngn) || 1,
            usd_kes: parseFloat(rate.usd_kes) || 0,
            usd_ghs: parseFloat(rate.usd_ghs) || 0,
            usd_zar: parseFloat(rate.usd_zar) || 0,
            usd_egp: parseFloat(rate.usd_egp) || 0,
          }
        : null,
    [rate]
  )

  const ngnFromDisplay = React.useCallback(
    (localAmount: number): number => {
      if (!fxRates) return localAmount
      return convertToNGN(localAmount, currency, fxRates)
    },
    [currency, fxRates]
  )

  const localFromNgn = React.useCallback(
    (ngnAmount: number): number => {
      if (!fxRates) return ngnAmount
      return convertFromNGN(ngnAmount, currency, fxRates)
    },
    [currency, fxRates]
  )

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

  const selectedUtility = UTILITIES.find((utility) => utility.id === selectedUtilityId)
  const isSchoolFees = selectedUtility?.type === "school"

  const filteredUtilities = UTILITIES.filter((utility) => {
    if (isGift && utility.type === "school") return false
    if (utility.type !== activeCategory) return false
    if (!query.trim()) return true
    const haystack = [utility.name, utility.descriptor, ...(utility.keywords ?? [])]
      .join(" ")
      .toLowerCase()
    return haystack.includes(query.trim().toLowerCase())
  })

  const effectiveAmount = React.useMemo(() => {
    if (!selectedUtility) return amountNgn
    if (selectedUtility.fixedAmountKobo) return selectedUtility.fixedAmountKobo / 100
    if (selectedUtility.hasVariations && variationCode) {
      const variation = variations.find((item) => item.variation_code === variationCode)
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
    if (currency === "NGN" || !fxRates) {
      setDisplayAmount(String(amountNgn))
    } else {
      setDisplayAmount(localFromNgn(amountNgn).toFixed(2))
    }
  }, [amountNgn, currency, fxRates, localFromNgn])

  const handleDisplayAmountChange = React.useCallback(
    (raw: string) => {
      setDisplayAmount(raw)
      const parsed = parseFloat(raw)
      if (isNaN(parsed) || parsed <= 0) {
        setValue("amountNgn", 0, { shouldValidate: true })
        return
      }
      if (currency === "NGN" || !fxRates) {
        setValue("amountNgn", Math.round(parsed), { shouldValidate: true })
      } else {
        setValue("amountNgn", Math.round(ngnFromDisplay(parsed)), { shouldValidate: true })
      }
    },
    [currency, fxRates, ngnFromDisplay, setValue]
  )

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
    const queriedReference = serviceRef.trim()
    setValidating(true)
    try {
      const result = await apiGet<UtilityValidateResponse>(
        `/api/v1/utilities/${selectedUtility.slug}/validate?ref=${encodeURIComponent(serviceRef)}`
      )
      setValidated(result)
      setValidatedReference(queriedReference)
      if (!result.valid) {
        toast.error(
          selectedUtility.type === "school"
            ? "That RRR is not payable"
            : "That account number did not validate"
        )
        return
      }

      if (result.amount_ngn) {
        setValue("amountNgn", result.amount_ngn, { shouldValidate: true })
      }

      toast.success(
        selectedUtility.type === "school"
          ? result.customer_name ?? "RRR found"
          : result.customer_name
            ? `Account name: ${result.customer_name}`
            : "Account number validated"
      )
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Validation failed")
    } finally {
      setValidating(false)
    }
  }, [selectedUtility, serviceRef, setValue])

  React.useEffect(() => {
    setValidated(null)
    setValidatedReference(null)
  }, [serviceRef, selectedUtilityId])

  const onSubmit = async (data: OrderFormValues) => {
    if (!selectedUtility) return

    const amountNgn = Math.round(effectiveAmount)

    if (
      isSchoolFees &&
      (!validated?.valid || validatedReference !== data.serviceRef.trim())
    ) {
      toast.error("Look up the RRR again")
      return
    }

    if (!isGift && !data.serviceRef.trim()) {
      toast.error("Enter the account number")
      return
    }

    if (isGift && selectedUtility.type === "school") {
      toast.error("School fees cannot be gifted yet")
      return
    }

    if (isGift && amountNgn > GIFT_MAX_NGN) {
      toast.error(`Gifts are capped at ${formatCurrency(GIFT_MAX_NGN, "NGN")}`)
      return
    }

    if (isGift && selectedUtility.type === "electricity" && !giftFallbackRef.trim()) {
      toast.error("Electricity gifts need a fallback meter number")
      return
    }

    setIsLoading(true)
    try {
      const utilityType =
        selectedUtility.type === "school"
          ? "school_fees"
          : selectedUtility.type === "tv"
            ? selectedUtility.slug
            : selectedUtility.slug === "waec-registration" ||
                selectedUtility.slug === "waec-result-checker"
              ? "waec"
              : selectedUtility.slug === "jamb"
                ? "jamb"
                : selectedUtility.type

      if (isGift) {
        const gift = await apiPost<CreateGiftResponse>("/api/v1/gifts", {
          utility_type: utilityType,
          utility_slug: selectedUtility.slug,
          amount_ngn: amountNgn,
          zec_address_type: data.addressType,
          variation_code: data.variationCode || undefined,
          message: giftMessage.trim() || undefined,
          fallback_service_ref: giftFallbackRef.trim() || undefined,
          claim_window_hours: giftWindowHours,
          hide_sender_name: giftHideName,
        })

        setActiveOrder(gift.order, user?.id ?? null)
        toast.success("Gift created")
        router.push(`/pay/${gift.order.order_id}?gift=${gift.gift_code}`)
        return
      }

      const order = await apiPost<CreateOrderResponse>("/api/v1/orders/create", {
        utility_type: utilityType,
        utility_slug: selectedUtility.slug,
        service_ref: data.serviceRef,
        amount_ngn: amountNgn,
        zec_address_type: data.addressType,
        variation_code: data.variationCode || undefined,
      })

      setActiveOrder(order, user?.id ?? null)
      toast.success("Order created")
      router.push(`/pay/${order.order_id}`)
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : isGift
            ? "Could not create the gift"
            : "Could not create the order"
      )
    } finally {
      setIsLoading(false)
    }
  }

  const canSubmit =
    Boolean(selectedUtilityId) &&
    effectiveAmount > 0 &&
    (!isSchoolFees || Boolean(validated?.valid))

  return (
    <div className="space-y-7">
      <header>
        <p className="label-xs text-muted">{isGift ? "New gift" : "New order"}</p>
        <h1 className="mt-2 text-3xl text-ink">{isGift ? "Send a utility gift" : "Pay a bill"}</h1>
        <p className="mt-1.5 max-w-lg text-[14px] leading-relaxed text-muted">
          {isGift
            ? "Pick the utility and the amount, then share one link. The recipient enters their own account, so you never need their number."
            : "Choose the service, enter the account, and send the quoted ZEC amount. No identity documents are needed."}
        </p>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => setIsGift(false)}
            className={cn(
              "rounded-md border px-3 py-1.5 text-[12px] transition-colors",
              !isGift ? "border-ink bg-cream text-ink" : "border-line text-muted hover:text-ink"
            )}
          >
            Pay for myself
          </button>
          <button
            type="button"
            onClick={() => {
              setIsGift(true)
              if (selectedUtility?.type === "school") {
                setValue("utilityId", "", { shouldValidate: true })
              }
            }}
            className={cn(
              "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-[12px] transition-colors",
              isGift ? "border-ink bg-cream text-ink" : "border-line text-muted hover:text-ink"
            )}
          >
            <Gift className="h-3.5 w-3.5" />
            Send as a gift
          </button>
        </div>
      </header>

      <form onSubmit={handleSubmit(onSubmit)} className="grid gap-8 lg:grid-cols-[1fr_280px]">
        <div className="min-w-0 space-y-8">
          <section>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Service</h2>
              <span className="num text-[11px] text-muted">Step 1 of 3</span>
            </div>

            <div className="mt-4 flex flex-wrap items-center gap-1">
              {UTILITY_CATEGORIES.map((category) => (
                <button
                  key={category.type}
                  type="button"
                  onClick={() => setActiveCategory(category.type)}
                  className={cn(
                    "rounded-md px-2.5 py-1 text-[12px] transition-colors",
                    activeCategory === category.type
                      ? "bg-ink text-cream"
                      : "text-muted hover:bg-paper-3/60 hover:text-ink"
                  )}
                >
                  {category.label}
                </button>
              ))}
            </div>

            <div className="relative mt-4">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search this category"
                leftIcon={<Search className="h-4 w-4" />}
                className="pl-9"
              />
            </div>

            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              {filteredUtilities.map((utility) => {
                const isSelected = selectedUtilityId === utility.id
                return (
                  <button
                    key={utility.id}
                    type="button"
                    onClick={() => setValue("utilityId", utility.id, { shouldValidate: true })}
                    className={cn(
                      "group/utility flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-[transform,border-color,background-color] duration-200 ease-out hover:-translate-y-0.5 hover:border-line-strong",
                      isSelected
                        ? "border-ink bg-cream"
                        : "border-line bg-cream/50 hover:border-line-strong"
                    )}
                  >
                    <span className="transition-transform duration-200 ease-out group-hover/utility:scale-110">
                      <BrandTile brandIcon={utility.brandIcon} selected={isSelected} />
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[13px] font-medium text-ink">
                        {utility.name}
                      </span>
                      <span className="block truncate text-[11px] text-muted">
                        {utility.descriptor}
                      </span>
                    </span>
                    {isSelected && <Check className="h-4 w-4 shrink-0 text-gold-deep" />}
                  </button>
                )
              })}
              {filteredUtilities.length === 0 && (
                <p className="col-span-full rounded-lg border border-dashed border-line-strong px-3 py-6 text-center text-[13px] text-muted">
                  Nothing matches that search.
                </p>
              )}
            </div>
            {errors.utilityId && (
              <p className="mt-2 text-[12px] text-rust">{errors.utilityId.message}</p>
            )}
          </section>

          <section className={cn(!selectedUtilityId && "opacity-55")}>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Account</h2>
              <span className="num text-[11px] text-muted">Step 2 of 3</span>
            </div>

            <div className="mt-4 space-y-4">
              {isGift ? (
                <p className="rounded-md border border-dashed border-line-strong px-3 py-2.5 text-[12px] leading-relaxed text-muted">
                  The recipient enters {selectedUtility?.serviceRefLabel?.toLowerCase() ?? "their account"} when they claim the link.
                </p>
              ) : (
                <div className="space-y-1.5">
                  <div className="flex items-baseline justify-between">
                    <label htmlFor="serviceRef" className="text-[13px] font-medium text-ink">
                      {selectedUtility?.serviceRefLabel || "Account number"}
                    </label>
                    {serviceRef && serviceRef.length >= 5 && (
                      <button
                        type="button"
                        onClick={handleValidate}
                        disabled={validating}
                        className="flex items-center gap-1 text-[12px] text-gold-deep u-link disabled:opacity-50"
                      >
                        {validating && <Loader2 className="h-3 w-3 animate-spin" />}
                        {validated ? "Check again" : isSchoolFees ? "Look up the RRR" : "Check the account"}
                      </button>
                    )}
                  </div>
                  <Input
                    id="serviceRef"
                    {...register("serviceRef")}
                    placeholder={selectedUtility?.serviceRefPlaceholder || "Enter the number"}
                    error={errors.serviceRef?.message}
                    disabled={!selectedUtilityId}
                  />
                  {isSchoolFees && !validated && (
                    <p className="text-[12px] text-muted">
                      Your RRR names the school. We show it after the lookup.
                    </p>
                  )}
                  {validated && !validated.valid && (
                    <p className="flex items-center gap-1.5 text-[12px] text-rust">
                      <Info className="h-3.5 w-3.5" />
                      {isSchoolFees ? "That RRR is not payable" : "That number did not validate"}
                    </p>
                  )}

                  {validated?.valid && isSchoolFees && (
                    <div className="rounded-md border border-line bg-paper px-3 py-3">
                      <p className="label-xs text-muted">RRR details</p>
                      <p className="mt-1 text-[13px] text-ink">
                        {validated.biller_name ?? "Remita biller"}
                      </p>
                      {validated.product_name && (
                        <p className="text-[12px] text-muted">{validated.product_name}</p>
                      )}
                      {validated.customer_name && (
                        <p className="mt-1 text-[12px] text-ink-2">{validated.customer_name}</p>
                      )}
                      <p className="num mt-2 text-lg text-ink">
                        {formatCurrency(validated.amount_ngn ?? 0, "NGN")}
                      </p>
                    </div>
                  )}

                  {validated?.valid && !isSchoolFees && (
                    <p className="flex items-center gap-1.5 text-[12px] text-green">
                      <Check className="h-3.5 w-3.5" />
                      {validated.customer_name
                        ? `Account name: ${validated.customer_name}`
                        : "Account number validated"}
                    </p>
                  )}
                </div>
              )}

              {selectedUtility?.hasVariations && (
                <div className="space-y-1.5">
                  <p className="text-[13px] font-medium text-ink">Plan</p>
                  {loadingVariations ? (
                    <p className="flex items-center gap-2 text-[13px] text-muted">
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      Loading plans
                    </p>
                  ) : variations.length > 0 ? (
                    <div className="max-h-64 space-y-1.5 overflow-y-auto pr-1">
                      {variations.map((variation) => {
                        const isSelected = variationCode === variation.variation_code
                        return (
                          <button
                            key={variation.variation_code}
                            type="button"
                            onClick={() => {
                              setValue("variationCode", variation.variation_code, {
                                shouldValidate: true,
                              })
                              if (variation.amount) {
                                setValue("amountNgn", variation.amount, { shouldValidate: true })
                              }
                            }}
                            className={cn(
                              "flex w-full items-baseline justify-between gap-3 rounded-md border px-3 py-2 text-left transition-[border-color,background-color] duration-150",
                              isSelected
                                ? "border-ink bg-cream"
                                : "border-line bg-cream/50 hover:border-line-strong hover:bg-cream"
                            )}
                          >
                            <span className="text-[13px] text-ink">{variation.name}</span>
                            {variation.amount && (
                              <span className="num text-[12px] text-muted">
                                {fxRates
                                  ? formatLocalAmount(variation.amount, currency, fxRates)
                                  : formatCurrency(variation.amount, "NGN")}
                              </span>
                            )}
                          </button>
                        )
                      })}
                    </div>
                  ) : (
                    <p className="text-[13px] text-muted">No plans returned for this service.</p>
                  )}
                </div>
              )}

              {selectedUtility?.hasAmountPicker && !selectedUtility.fixedAmountKobo && (
                <div className="space-y-1.5">
                  <label htmlFor="amount" className="text-[13px] font-medium text-ink">
                    Amount
                  </label>
                  <Input
                    id="amount"
                    type="number"
                    value={displayAmount}
                    onChange={(event) => handleDisplayAmountChange(event.target.value)}
                    leftIcon={<span className="num text-[13px] text-muted">{currencySymbol}</span>}
                    error={errors.amountNgn?.message}
                    disabled={!selectedUtilityId}
                  />
                  {selectedUtility.quickAmounts && (
                    <div className="flex flex-wrap gap-1.5 pt-1">
                      {selectedUtility.quickAmounts.map((quickAmount) => (
                        <button
                          key={quickAmount}
                          type="button"
                          onClick={() =>
                            setValue("amountNgn", quickAmount, { shouldValidate: true })
                          }
                          className={cn(
                            "num rounded-md border px-2.5 py-1 text-[12px] transition-colors",
                            amountNgn === quickAmount
                              ? "border-ink bg-cream text-ink"
                              : "border-line text-muted hover:text-ink"
                          )}
                        >
                          {fxRates
                            ? formatLocalAmount(quickAmount, currency, fxRates)
                            : formatCurrency(quickAmount, "NGN")}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {isSchoolFees && errors.amountNgn && (
                <p className="text-[12px] text-rust">{errors.amountNgn.message}</p>
              )}

              {selectedUtility?.fixedAmountKobo && (
                <div className="rounded-md border border-line bg-paper px-3 py-2.5">
                  <p className="label-xs text-muted">Fixed amount</p>
                  <p className="num mt-1 text-xl text-ink">
                    {fxRates
                      ? formatLocalAmount(selectedUtility.fixedAmountKobo / 100, currency, fxRates)
                      : formatCurrency(selectedUtility.fixedAmountKobo / 100, "NGN")}
                  </p>
                </div>
              )}
            </div>
          </section>

          <section className={cn(!selectedUtilityId && "opacity-55")}>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Payment address</h2>
              <span className="num text-[11px] text-muted">Step 3 of 3</span>
            </div>

            <div className="mt-4 grid gap-2 sm:grid-cols-2">
              {[
                {
                  value: "shielded" as const,
                  title: "Shielded",
                  hint: "z-address. 2 confirmations, about a minute.",
                  icon: ShieldCheck,
                },
                {
                  value: "transparent" as const,
                  title: "Transparent",
                  hint: "t-address. 2 confirmations, about a minute.",
                  icon: null,
                },
              ].map((option) => {
                const isSelected = addressType === option.value
                return (
                  <button
                    key={option.value}
                    type="button"
                    disabled={!selectedUtilityId}
                    onClick={() => setValue("addressType", option.value)}
                    className={cn(
                      "rounded-lg border px-3 py-3 text-left transition-[border-color,background-color,transform] duration-200 hover:-translate-y-0.5",
                      isSelected ? "border-ink bg-cream" : "border-line bg-cream/50 hover:border-line-strong"
                    )}
                  >
                    <span className="flex items-center justify-between">
                      <span className="flex items-center gap-1.5 text-[13px] font-medium text-ink">
                        {option.icon && <option.icon className="h-3.5 w-3.5 text-gold-deep" />}
                        {option.title}
                      </span>
                      <span
                        className={cn(
                          "flex h-3.5 w-3.5 items-center justify-center rounded-full border",
                          isSelected ? "border-ink" : "border-line-strong"
                        )}
                      >
                        {isSelected && <span className="h-1.5 w-1.5 rounded-full bg-ink" />}
                      </span>
                    </span>
                    <span className="mt-1 block text-[11px] leading-relaxed text-muted">
                      {option.hint}
                    </span>
                  </button>
                )
              })}
            </div>
          </section>

          {isGift && (
            <section>
              <div className="flex items-baseline justify-between border-b border-line pb-2">
                <h2 className="text-lg text-ink">Gift details</h2>
                <span className="num text-[11px] text-muted">The good part</span>
              </div>

              <div className="mt-4 space-y-4">
                <div className="space-y-1.5">
                  <div className="flex items-baseline justify-between">
                    <label htmlFor="giftMessage" className="text-[13px] font-medium text-ink">
                      Message
                    </label>
                    <span className="num text-[11px] text-muted">{giftMessage.length}/240</span>
                  </div>
                  <textarea
                    id="giftMessage"
                    value={giftMessage}
                    onChange={(event) => setGiftMessage(event.target.value.slice(0, 240))}
                    rows={2}
                    placeholder="For your data. Stay online."
                    className="w-full rounded-md border border-line bg-cream px-3 py-2 text-[13px] text-ink outline-none transition-colors focus:border-ink"
                  />
                </div>

                <div className="space-y-1.5">
                  <p className="text-[13px] font-medium text-ink">Claim window</p>
                  <div className="flex flex-wrap gap-1.5">
                    {GIFT_WINDOWS.map((option) => (
                      <button
                        key={option.hours}
                        type="button"
                        onClick={() => setGiftWindowHours(option.hours)}
                        className={cn(
                          "num rounded-md border px-2.5 py-1 text-[12px] transition-colors",
                          giftWindowHours === option.hours
                            ? "border-ink bg-cream text-ink"
                            : "border-line text-muted hover:text-ink"
                        )}
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                  <p className="text-[11px] leading-relaxed text-muted">
                    The clock starts when your payment confirms.
                  </p>
                </div>

                <div className="space-y-1.5">
                  <label htmlFor="giftFallback" className="text-[13px] font-medium text-ink">
                    If nobody claims it{" "}
                    {selectedUtility?.type === "electricity" ? "(required)" : "(optional)"}
                  </label>
                  <Input
                    id="giftFallback"
                    value={giftFallbackRef}
                    onChange={(event) => setGiftFallbackRef(event.target.value)}
                    placeholder={selectedUtility?.serviceRefPlaceholder ?? "Your own number"}
                  />
                  <p className="text-[11px] leading-relaxed text-muted">
                    We deliver the utility here on the deadline if the gift is still unclaimed.
                  </p>
                </div>

                <label className="flex items-center gap-2 text-[13px] text-ink">
                  <input
                    type="checkbox"
                    checked={giftHideName}
                    onChange={(event) => setGiftHideName(event.target.checked)}
                    className="h-4 w-4 rounded border-line"
                  />
                  Hide my name on the gift
                </label>

                <p className="text-[11px] leading-relaxed text-muted">
                  Gifts are capped at {formatCurrency(GIFT_MAX_NGN, "NGN")}. A gift link can be
                  claimed once, by the first person who opens it.
                </p>
              </div>
            </section>
          )}
        </div>

        <aside className="lg:sticky lg:top-8 lg:self-start">
          <div className="rounded-lg border border-line bg-cream">
            <div className="border-b border-line px-4 py-3">
              <p className="label-xs text-muted">{isGift ? "Gift summary" : "Order summary"}</p>
            </div>
            <div className="space-y-3 px-4 py-4">
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Service</span>
                <span className="truncate text-right text-[13px] text-ink">
                  {selectedUtility?.name ?? "Not selected"}
                </span>
              </div>
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Amount</span>
                <span className="num text-[13px] text-ink">
                  {selectedUtility && effectiveAmount > 0
                    ? fxRates
                      ? formatLocalAmount(effectiveAmount, currency, fxRates)
                      : formatCurrency(effectiveAmount, "NGN")
                    : "---"}
                </span>
              </div>
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Address</span>
                <span className="text-[13px] capitalize text-ink">{addressType}</span>
              </div>

              {isGift && (
                <div className="flex items-baseline justify-between gap-3">
                  <span className="text-[13px] text-muted">Claim window</span>
                  <span className="text-[13px] text-ink">
                    {GIFT_WINDOWS.find((option) => option.hours === giftWindowHours)?.label}
                  </span>
                </div>
              )}

              <div className="border-t border-line pt-3">
                <p className="text-[13px] text-muted">You send</p>
                <p className="num mt-1 text-2xl text-ink">
                  {selectedUtility ? `${estimatedZec} ZEC` : "---"}
                </p>
                <p className="mt-1 text-[11px] leading-relaxed text-muted">
                  The order page shows the exact amount and the deposit address. The quote is
                  fixed when the order is created.
                </p>
              </div>

              <Button
                type="submit"
                className="w-full"
                size="lg"
                loading={isLoading}
                disabled={!canSubmit}
              >
                {isGift ? "Create gift" : "Create order"}
              </Button>

              <p className="text-[11px] leading-relaxed text-muted">
                {isGift
                  ? "After payment you get one link. Share it. The recipient claims the gift with their own account."
                  : "Delivery follows the network confirmations. Shielded and transparent payments confirm in about the same time."}
              </p>
            </div>
          </div>
        </aside>
      </form>
    </div>
  )
}
