"use client"

import { Loader2 } from "lucide-react"
import { Input } from "@/components/ui/input"
import { cn } from "@/lib/utils"
import { formatAmount, isCurrencyCode } from "@/lib/currency"
import { quickAmounts } from "@/lib/catalogue"
import type { CatalogueProduct } from "@/lib/types"

interface AmountControlProps {
  product: CatalogueProduct
  variationCode: string | null
  onVariationChange: (code: string) => void
  amount: string
  onAmountChange: (value: string) => void
  disabled?: boolean
  error?: string | null
}

function formatProductAmount(product: CatalogueProduct, value: string | null): string {
  if (value === null) return "—"
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) return `${value} ${product.currency}`
  return isCurrencyCode(product.currency)
    ? formatAmount(parsed, product.currency)
    : `${value} ${product.currency}`
}

export function AmountControl({
  product,
  variationCode,
  onVariationChange,
  amount,
  onAmountChange,
  disabled,
  error,
}: AmountControlProps) {
  if (product.amount_kind === "variation") {
    return (
      <div className="space-y-1.5">
        <p className="text-[13px] font-medium text-ink">Plan</p>
        {product.variations.length > 0 ? (
          <div className="max-h-64 space-y-1.5 overflow-y-auto pr-1">
            {product.variations.map((variation) => {
              const isSelected = variationCode === variation.variation_code
              return (
                <button
                  key={variation.variation_code}
                  type="button"
                  disabled={disabled}
                  onClick={() => onVariationChange(variation.variation_code)}
                  className={cn(
                    "flex w-full items-baseline justify-between gap-3 rounded-md border px-3 py-2 text-left transition-[border-color,background-color] duration-150 disabled:opacity-50",
                    isSelected
                      ? "border-ink bg-cream"
                      : "border-line bg-cream/50 hover:border-line-strong hover:bg-cream"
                  )}
                >
                  <span className="text-[13px] text-ink">{variation.name}</span>
                  <span className="num text-[12px] text-muted">
                    {formatProductAmount(product, variation.amount)}
                  </span>
                </button>
              )
            })}
          </div>
        ) : (
          <p className="flex items-center gap-2 text-[13px] text-muted">
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
            No plans returned for this service yet.
          </p>
        )}
        {error && <p className="text-[12px] text-rust">{error}</p>}
      </div>
    )
  }

  if (product.amount_kind === "fixed") {
    return (
      <div className="rounded-md border border-line bg-paper px-3 py-2.5">
        <p className="label-xs text-muted">Fixed amount</p>
        <p className="num mt-1 text-xl text-ink">
          {formatProductAmount(product, product.amount_min)}
        </p>
      </div>
    )
  }

  const minimum = product.amount_min ?? "0"
  const maximum = product.amount_max ?? "0"
  const increment = product.amount_increment
  const suggestions = quickAmounts(product)

  return (
    <div className="space-y-1.5">
      <label htmlFor="amount" className="text-[13px] font-medium text-ink">
        Amount
      </label>
      <Input
        id="amount"
        type="number"
        inputMode="decimal"
        min={minimum}
        max={maximum}
        step={increment ?? "1"}
        value={amount}
        onChange={(event) => onAmountChange(event.target.value)}
        leftIcon={<span className="num text-[12px] text-muted">{product.currency}</span>}
        error={error ?? undefined}
        disabled={disabled}
      />
      <p className="num text-[11px] text-muted">
        {formatProductAmount(product, minimum)} to {formatProductAmount(product, maximum)}
        {increment ? `, steps of ${formatProductAmount(product, increment)}` : ""}
      </p>
      {suggestions.length > 0 && (
        <div className="flex flex-wrap gap-1.5 pt-1">
          {suggestions.map((suggestion) => (
            <button
              key={suggestion}
              type="button"
              disabled={disabled}
              onClick={() => onAmountChange(String(suggestion))}
              className={cn(
                "num rounded-md border px-2.5 py-1 text-[12px] transition-colors disabled:opacity-50",
                Number(amount) === suggestion
                  ? "border-ink bg-cream text-ink"
                  : "border-line text-muted hover:text-ink"
              )}
            >
              {formatProductAmount(product, String(suggestion))}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
