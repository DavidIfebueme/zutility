"use client"

import { cn } from "@/lib/utils"
import type { CatalogueCountry } from "@/lib/types"

interface CountryPickerProps {
  countries: CatalogueCountry[]
  value: string | null
  onChange: (iso2: string) => void
  disabled?: boolean
}

export function CountryPicker({ countries, value, onChange, disabled }: CountryPickerProps) {
  if (countries.length === 0) {
    return (
      <p className="rounded-md border border-dashed border-line-strong px-3 py-2.5 text-[12px] text-muted">
        No country has an enabled product yet.
      </p>
    )
  }

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {countries.map((country) => {
        const isSelected = country.iso2 === value
        return (
          <button
            key={country.iso2}
            type="button"
            disabled={disabled}
            aria-pressed={isSelected}
            onClick={() => onChange(country.iso2)}
            className={cn(
              "flex items-baseline gap-1.5 rounded-md border px-2.5 py-1 text-[12px] transition-colors disabled:opacity-50",
              isSelected
                ? "border-ink bg-cream text-ink"
                : "border-line text-muted hover:border-line-strong hover:text-ink"
            )}
          >
            <span>{country.name}</span>
            <span className="num text-[10px] text-muted">{country.currency}</span>
          </button>
        )
      })}
    </div>
  )
}
