"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import * as z from "zod"
import {
  Check,
  Gift,
  GraduationCap,
  Info,
  Loader2,
  Receipt,
  School,
  Search,
  ShieldCheck,
  Smartphone,
  Tv,
  Wifi,
  Zap,
  type LucideIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { BrandMark } from "@/components/ui/brand-mark"
import { CountryPicker } from "@/components/pay/country-picker"
import { ProductFields } from "@/components/pay/product-fields"
import { AmountControl } from "@/components/pay/amount-control"
import { useOrderStore } from "@/store/order"
import { useAuthStore } from "@/store/auth"
import { useRate } from "@/lib/hooks/useRate"
import { apiGet, apiPost } from "@/lib/api"
import {
  CreateGiftResponse,
  CreateOrderResponse,
  UtilityValidateResponse,
} from "@/lib/types"
import type {
  CatalogueProduct,
  CatalogueResponse,
  CatalogueVariationItem,
} from "@/lib/types"
import { formatAmount, isCurrencyCode } from "@/lib/currency"
import {
  categoriesWithProducts,
  categoryLabel,
  categoryUtilityType,
  countriesWithProducts,
  fieldLabel,
  findProduct,
  isPhoneField,
  needsLiveVariations,
  requiredFieldNames,
  resolveAmount,
  variationsFromLive,
} from "@/lib/catalogue"
import { toast } from "sonner"
import { cn } from "@/lib/utils"
import { networkWarning } from "@/lib/network"
import { INVALID_PHONE_MESSAGE, normalizePhone } from "@/lib/phone"
import { brandIconForSlug } from "@/lib/brand-logo"

const createOrderSchema = z.object({
  utility_type: z.string().min(1),
  utility_slug: z.string().min(1),
  service_ref: z.string().min(1),
  zec_address_type: z.literal("shielded"),
  product_id: z.string().min(1),
  country_iso2: z.string().length(2),
  variation_code: z.string().optional(),
  settlement: z.object({
    currency: z.string().length(3),
    amount: z.string().min(1),
  }),
  inputs: z.record(z.string(), z.string()),
})

const GIFT_MAX_NGN = 100_000
const GIFT_WINDOWS = [
  { hours: 12, label: "12 hours" },
  { hours: 24, label: "24 hours" },
  { hours: 72, label: "3 days" },
  { hours: 168, label: "7 days" },
]

const CATEGORY_ICONS: Record<string, LucideIcon> = {
  airtime: Smartphone,
  data: Wifi,
  television: Tv,
  electricity: Zap,
  education: GraduationCap,
  school: School,
}

function categoryIcon(category: string): LucideIcon {
  return CATEGORY_ICONS[category] ?? Receipt
}

function formatMoney(amount: string, currency: string): string {
  const value = Number(amount)
  if (isCurrencyCode(currency) && Number.isFinite(value)) {
    return formatAmount(value, currency)
  }
  return `${amount} ${currency}`
}

export default function PayPage() {
  const router = useRouter()
  const { setActiveOrder } = useOrderStore()
  const { user } = useAuthStore()
  const { rate } = useRate()

  const [catalogue, setCatalogue] = React.useState<CatalogueResponse | null>(null)
  const [catalogueError, setCatalogueError] = React.useState(false)
  const [liveVariations, setLiveVariations] = React.useState<
    Record<string, CatalogueVariationItem[]>
  >({})
  const [countryIso, setCountryIso] = React.useState<string | null>(null)
  const [activeCategory, setActiveCategory] = React.useState<string | null>(null)
  const [selectedProductId, setSelectedProductId] = React.useState<string | null>(null)
  const [query, setQuery] = React.useState("")
  const [serviceRef, setServiceRef] = React.useState("")
  const [inputs, setInputs] = React.useState<Record<string, string>>({})
  const [variationCode, setVariationCode] = React.useState<string | null>(null)
  const [amountInput, setAmountInput] = React.useState("")
  const [amountError, setAmountError] = React.useState<string | null>(null)
  const [validating, setValidating] = React.useState(false)
  const [validated, setValidated] = React.useState<UtilityValidateResponse | null>(null)
  const [isLoading, setIsLoading] = React.useState(false)
  const [isGift, setIsGift] = React.useState(false)
  const [giftMessage, setGiftMessage] = React.useState("")
  const [giftFallbackRef, setGiftFallbackRef] = React.useState("")
  const [giftWindowHours, setGiftWindowHours] = React.useState(168)
  const [giftHideName, setGiftHideName] = React.useState(false)

  const countries = React.useMemo(() => countriesWithProducts(catalogue), [catalogue])
  const country = countries.find((item) => item.iso2 === countryIso) ?? null
  const categories = React.useMemo(() => {
    const list = categoriesWithProducts(country)
    return isGift ? list.filter((item) => item.category !== "school") : list
  }, [country, isGift])
  const activeCategoryEntry = categories.find((item) => item.category === activeCategory) ?? null
  const selected = findProduct(country, selectedProductId)
  const catalogueProduct = selected?.product ?? null
  const liveSlug =
    catalogueProduct && needsLiveVariations(catalogueProduct) ? catalogueProduct.slug : null
  const liveRows = liveSlug ? liveVariations[liveSlug] : undefined
  const product =
    catalogueProduct && liveRows ? { ...catalogueProduct, variations: liveRows } : catalogueProduct
  const productCategory = selected?.category ?? null
  const isSchoolFees = productCategory === "school"
  const requiredNames = product ? requiredFieldNames(product) : []
  const usesRequiredFields = requiredNames.length > 0
  const primaryReference = usesRequiredFields ? inputs[requiredNames[0]] ?? "" : serviceRef

  React.useEffect(() => {
    if (new URLSearchParams(window.location.search).get("gift") !== "1") return
    const timer = window.setTimeout(() => setIsGift(true), 0)
    return () => window.clearTimeout(timer)
  }, [])

  React.useEffect(() => {
    let cancelled = false
    apiGet<CatalogueResponse>("/api/v1/catalogue")
      .then((data) => {
        if (cancelled) return
        setCatalogue(data)
        const list = countriesWithProducts(data)
        const first = list.find((item) => item.iso2 === "NG") ?? list[0] ?? null
        if (first) {
          setCountryIso(first.iso2)
          const nextCategories = categoriesWithProducts(first)
          setActiveCategory(nextCategories[0]?.category ?? null)
        }
      })
      .catch(() => {
        if (!cancelled) setCatalogueError(true)
      })
    return () => {
      cancelled = true
    }
  }, [])

  React.useEffect(() => {
    if (!liveSlug) return
    if (liveVariations[liveSlug] !== undefined) return
    let cancelled = false
    apiGet<unknown>(`/api/v1/utilities/${encodeURIComponent(liveSlug)}/variations`)
      .then((rows) => {
        if (cancelled) return
        const items = variationsFromLive(rows)
        setLiveVariations((current) =>
          current[liveSlug] === undefined ? { ...current, [liveSlug]: items } : current
        )
      })
      .catch(() => {
        if (cancelled) return
        setLiveVariations((current) =>
          current[liveSlug] === undefined ? { ...current, [liveSlug]: [] } : current
        )
      })
    return () => {
      cancelled = true
    }
  }, [liveSlug, liveVariations])

  const resetProductState = React.useCallback(() => {
    setSelectedProductId(null)
    setServiceRef("")
    setInputs({})
    setVariationCode(null)
    setAmountInput("")
    setAmountError(null)
    setValidated(null)
  }, [])

  const chooseCountry = (iso2: string) => {
    setCountryIso(iso2)
    resetProductState()
    const next = categoriesWithProducts(countries.find((item) => item.iso2 === iso2) ?? null)
    setActiveCategory(next[0]?.category ?? null)
  }

  const chooseCategory = (category: string) => {
    setActiveCategory(category)
    resetProductState()
  }

  const chooseProduct = (next: CatalogueProduct) => {
    setSelectedProductId(next.id)
    setServiceRef("")
    setInputs({})
    setVariationCode(null)
    setValidated(null)
    setAmountError(null)
    if (next.amount_kind === "fixed") {
      setAmountInput(next.amount_min ?? "")
    } else if (next.amount_kind === "ranged") {
      setAmountInput(next.amount_min ?? "")
    } else {
      setAmountInput("")
    }
  }

  const toggleGift = (next: boolean) => {
    setIsGift(next)
    if (next) {
      const nigeria = countries.find((item) => item.iso2 === "NG")
      if (nigeria) {
        setCountryIso(nigeria.iso2)
        resetProductState()
      }
    }
    router.replace(next ? "/pay?gift=1" : "/pay")
  }

  const filteredProducts = (activeCategoryEntry?.products ?? []).filter((item) => {
    if (!query.trim()) return true
    const haystack = [item.name, item.descriptor, item.operator_name ?? ""]
      .join(" ")
      .toLowerCase()
    return haystack.includes(query.trim().toLowerCase())
  })

  const selection = resolveAmount(product, variationCode, amountInput)

  const zecNgn = rate ? parseFloat(rate.zec_ngn) : 0
  const estimatedZec =
    product && selection.amount && product.currency === "NGN" && zecNgn > 0
      ? (Number(selection.amount) / zecNgn).toFixed(8)
      : null

  const refWarning = product ? networkWarning(primaryReference, product.slug) : null

  const handleValidate = async () => {
    if (!product || !primaryReference.trim()) return
    const reference = isPhoneField(product) ? normalizePhone(primaryReference) : primaryReference.trim()
    if (!reference) {
      toast.error(INVALID_PHONE_MESSAGE)
      return
    }
    setValidating(true)
    try {
      const result = await apiGet<UtilityValidateResponse>(
        `/api/v1/utilities/${product.slug}/validate?ref=${encodeURIComponent(reference)}`
      )
      setValidated(result)
      if (!result.valid) {
        toast.error(isSchoolFees ? "That RRR is not payable" : "That account did not validate")
        return
      }
      if (result.amount_ngn) {
        setAmountInput(String(result.amount_ngn))
      }
      toast.success(
        isSchoolFees
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
  }

  const submit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!product || !country || !productCategory) {
      toast.error("Select a service first")
      return
    }

    const selection = resolveAmount(product, variationCode, amountInput)
    if (selection.error || !selection.amount) {
      setAmountError(selection.error)
      toast.error(selection.error ?? "Enter an amount")
      return
    }

    if (isSchoolFees && !validated?.valid) {
      toast.error("Look up the RRR first")
      return
    }

    const missing = requiredFieldNames(product).find((name) => !inputs[name]?.trim())
    if (missing) {
      toast.error(`Enter ${fieldLabel(product, missing)}`)
      return
    }

    const rawReference = primaryReference.trim()
    const reference =
      !usesRequiredFields && isPhoneField(product) ? normalizePhone(rawReference) : rawReference
    if (!reference) {
      toast.error(isPhoneField(product) ? INVALID_PHONE_MESSAGE : "Enter the account number")
      return
    }

    if (isGift && Number(selection.amount) > GIFT_MAX_NGN) {
      toast.error(`Gifts are capped at ${formatAmount(GIFT_MAX_NGN, "NGN")}`)
      return
    }

    if (isGift && productCategory === "electricity" && !giftFallbackRef.trim()) {
      toast.error("Electricity gifts need a fallback meter number")
      return
    }

    let fallbackReference = giftFallbackRef.trim() || undefined
    if (fallbackReference && isPhoneField(product)) {
      const normalized = normalizePhone(fallbackReference)
      if (!normalized) {
        toast.error("Enter a valid mobile number for the backup account")
        return
      }
      fallbackReference = normalized
    }

    const trimmedInputs = Object.fromEntries(
      Object.entries(inputs)
        .map(([name, value]) => [name, value.trim()])
        .filter(([, value]) => value.length > 0)
    )

    setIsLoading(true)
    try {
      if (isGift) {
        const gift = await apiPost<CreateGiftResponse>("/api/v1/gifts", {
          utility_type: categoryUtilityType(productCategory),
          utility_slug: product.slug,
          amount_ngn: Math.round(Number(selection.amount)),
          zec_address_type: "shielded",
          variation_code: variationCode ?? undefined,
          message: giftMessage.trim() || undefined,
          fallback_service_ref: fallbackReference,
          claim_window_hours: giftWindowHours,
          hide_sender_name: giftHideName,
        })
        setActiveOrder(gift.order, user?.id ?? null)
        toast.success("Gift created")
        router.push(`/pay/${gift.order.order_id}?gift=${gift.gift_code}`)
        return
      }

      const payload = createOrderSchema.parse({
        utility_type: categoryUtilityType(productCategory),
        utility_slug: product.slug,
        service_ref: reference,
        zec_address_type: "shielded",
        product_id: product.id,
        country_iso2: country.iso2,
        variation_code: variationCode ?? undefined,
        settlement: { currency: product.currency, amount: selection.amount },
        inputs: trimmedInputs,
      })

      const order = await apiPost<CreateOrderResponse>("/api/v1/orders/create", payload)
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
    Boolean(product) &&
    Boolean(selection.amount) &&
    (!isSchoolFees || Boolean(validated?.valid))

  return (
    <div className="space-y-7">
      <header>
        <p className="label-xs text-muted">{isGift ? "New gift" : "New order"}</p>
        <h1 className="mt-2 text-3xl text-ink">{isGift ? "Send a utility gift" : "Pay a bill"}</h1>
        <p className="mt-1.5 max-w-lg text-[14px] leading-relaxed text-muted">
          {isGift
            ? "Pick the bill and the amount, then share one link. The recipient enters their own account, so you never need their number."
            : "Choose the country and the service, enter the account, and send the quoted ZEC amount. No identity documents are needed."}
        </p>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => toggleGift(false)}
            className={cn(
              "rounded-md border px-3 py-1.5 text-[12px] transition-colors",
              !isGift ? "border-ink bg-cream text-ink" : "border-line text-muted hover:text-ink"
            )}
          >
            Pay for myself
          </button>
          <button
            type="button"
            onClick={() => toggleGift(true)}
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

      <form onSubmit={submit} className="grid gap-8 lg:grid-cols-[1fr_280px]">
        <div className="min-w-0 space-y-8">
          <section>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Country</h2>
              <span className="num text-[11px] text-muted">Step 1 of 4</span>
            </div>
            <div className="mt-4 space-y-2">
              <CountryPicker
                countries={countries}
                value={countryIso}
                onChange={chooseCountry}
                disabled={isGift}
              />
              {isGift && (
                <p className="text-[11px] leading-relaxed text-muted">
                  Gifts are available in Nigeria only.
                </p>
              )}
              {catalogueError && (
                <p className="text-[12px] text-rust">
                  Could not load the catalogue. Reload the page to try again.
                </p>
              )}
            </div>
          </section>

          <section>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Service</h2>
              <span className="num text-[11px] text-muted">Step 2 of 4</span>
            </div>

            <div className="mt-4 flex flex-wrap items-center gap-1">
              {categories.map((category) => (
                <button
                  key={category.category}
                  type="button"
                  onClick={() => chooseCategory(category.category)}
                  className={cn(
                    "rounded-md px-2.5 py-1 text-[12px] transition-colors",
                    activeCategory === category.category
                      ? "bg-ink text-cream"
                      : "text-muted hover:bg-paper-3/60 hover:text-ink"
                  )}
                >
                  {categoryLabel(category.category)}
                </button>
              ))}
            </div>

            {!activeCategoryEntry && categories.length > 0 && (
              <p className="mt-3 rounded-md border border-dashed border-line-strong px-3 py-2.5 text-[12px] leading-relaxed text-muted">
                {country?.name ?? "This country"} has{" "}
                {categories.map((category) => categoryLabel(category.category)).join(", ")}. Pick
                one above to continue.
              </p>
            )}

            {activeCategoryEntry && (
              <>
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
                  {filteredProducts.map((item) => {
                    const Icon = categoryIcon(activeCategoryEntry.category)
                    const brandIcon = brandIconForSlug(item.slug)
                    const isSelected = selectedProductId === item.id
                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => chooseProduct(item)}
                        className={cn(
                          "group/utility flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-[transform,border-color,background-color] duration-200 ease-out hover:-translate-y-0.5 hover:border-line-strong",
                          isSelected
                            ? "border-ink bg-cream"
                            : "border-line bg-cream/50 hover:border-line-strong"
                        )}
                      >
                        <span
                          className={cn(
                            "flex h-10 w-10 shrink-0 items-center justify-center rounded-md transition-transform duration-200 ease-out group-hover/utility:scale-110",
                            isSelected ? "bg-paper-3" : "bg-paper-2"
                          )}
                        >
                          {brandIcon ? (
                            <BrandMark id={brandIcon} size={26} />
                          ) : (
                            <Icon className="h-5 w-5 text-ink-2" strokeWidth={1.6} />
                          )}
                        </span>
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-[13px] font-medium text-ink">
                            {item.name}
                          </span>
                          <span className="block truncate text-[11px] text-muted">
                            {item.descriptor}
                          </span>
                        </span>
                        {isSelected && <Check className="h-4 w-4 shrink-0 text-gold-deep" />}
                      </button>
                    )
                  })}
                  {filteredProducts.length === 0 && (
                    <p className="col-span-full rounded-lg border border-dashed border-line-strong px-3 py-6 text-center text-[13px] text-muted">
                      Nothing matches that search.
                    </p>
                  )}
                </div>
              </>
            )}
          </section>

          <section className={cn(!product && "opacity-55")}>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Account</h2>
              <span className="num text-[11px] text-muted">Step 3 of 4</span>
            </div>

            <div className="mt-4 space-y-4">
              {!product && (
                <p className="text-[13px] text-muted">Pick a service to see its fields.</p>
              )}

              {product && isGift && (
                <p className="rounded-md border border-dashed border-line-strong px-3 py-2.5 text-[12px] leading-relaxed text-muted">
                  You send the gift. The recipient enters{" "}
                  {product.service_ref_label.toLowerCase()} when they claim the link.
                </p>
              )}

              {product && !isGift && (
                <div className="space-y-2">
                  <div className="flex items-baseline justify-between">
                    <span className="text-[13px] font-medium text-ink">Check the account</span>
                    {country?.iso2 === "NG" && primaryReference.trim().length >= 5 && (
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
                  <ProductFields
                    product={product}
                    serviceRef={serviceRef}
                    onServiceRefChange={(value) => {
                      setServiceRef(value)
                      setValidated(null)
                      if (isSchoolFees) setAmountInput("")
                    }}
                    inputs={inputs}
                    onInputChange={(name, value) =>
                      setInputs((current) => ({ ...current, [name]: value }))
                    }
                  />
                  {refWarning && (
                    <p className="flex items-start gap-1.5 text-[12px] text-gold-deep">
                      <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                      {refWarning}
                    </p>
                  )}
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
                        {formatAmount(validated.amount_ngn ?? 0, "NGN")}
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

              {product && !isGift && !isSchoolFees && (
                <AmountControl
                  product={product}
                  variationCode={variationCode}
                  onVariationChange={(code) => {
                    setVariationCode(code)
                    setAmountError(null)
                  }}
                  amount={amountInput}
                  onAmountChange={(value) => {
                    setAmountInput(value)
                    setAmountError(null)
                  }}
                  error={amountError}
                />
              )}

              {product && isSchoolFees && (
                <p className="text-[11px] leading-relaxed text-muted">
                  The amount comes from the RRR. Look it up to see the exact total.
                </p>
              )}
            </div>
          </section>

          <section>
            <div className="flex items-baseline justify-between border-b border-line pb-2">
              <h2 className="text-lg text-ink">Payment address</h2>
              <span className="num text-[11px] text-muted">Step 4 of 4</span>
            </div>

            <div className="mt-4">
              <div className="rounded-lg border border-ink bg-cream px-3 py-3 text-left">
                <span className="flex items-center justify-between">
                  <span className="flex items-center gap-1.5 text-[13px] font-medium text-ink">
                    <ShieldCheck className="h-3.5 w-3.5 text-gold-deep" />
                    Shielded
                  </span>
                  <span className="flex h-3.5 w-3.5 items-center justify-center rounded-full border border-ink">
                    <span className="h-1.5 w-1.5 rounded-full bg-ink" />
                  </span>
                </span>
                <span className="mt-1 block text-[11px] leading-relaxed text-muted">
                  z-address. Confirms in about a minute.
                </span>
              </div>
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
                    {productCategory === "electricity" ? "(required)" : "(optional)"}
                  </label>
                  <Input
                    id="giftFallback"
                    value={giftFallbackRef}
                    onChange={(event) => setGiftFallbackRef(event.target.value)}
                    placeholder="Your own number"
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
                  Gifts are capped at {formatAmount(GIFT_MAX_NGN, "NGN")}. A gift link can be
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
                <span className="text-[13px] text-muted">Country</span>
                <span className="truncate text-right text-[13px] text-ink">
                  {country?.name ?? "Not selected"}
                </span>
              </div>
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Service</span>
                <span className="truncate text-right text-[13px] text-ink">
                  {product?.name ?? "Not selected"}
                </span>
              </div>
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Amount</span>
                <span className="num text-[13px] text-ink">
                  {product && selection.amount
                    ? formatMoney(selection.amount, product.currency)
                    : "---"}
                </span>
              </div>
              <div className="flex items-baseline justify-between gap-3">
                <span className="text-[13px] text-muted">Address</span>
                <span className="text-[13px] capitalize text-ink">shielded</span>
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
                  {estimatedZec ? `${estimatedZec} ZEC` : "Quoted on the order"}
                </p>
                <p className="mt-1 text-[11px] leading-relaxed text-muted">
                  The order page shows the exact ZEC amount and the deposit address. The quote is
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
                  : "Delivery follows the network confirmations. A shielded payment confirms in about a minute."}
              </p>
            </div>
          </div>
        </aside>
      </form>
    </div>
  )
}
