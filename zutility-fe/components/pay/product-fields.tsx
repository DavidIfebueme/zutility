"use client"

import { Input } from "@/components/ui/input"
import { fieldLabel, requiredFieldNames } from "@/lib/catalogue"
import type { CatalogueProduct } from "@/lib/types"

interface ProductFieldsProps {
  product: CatalogueProduct
  serviceRef: string
  onServiceRefChange: (value: string) => void
  inputs: Record<string, string>
  onInputChange: (name: string, value: string) => void
  disabled?: boolean
}

export function ProductFields({
  product,
  serviceRef,
  onServiceRefChange,
  inputs,
  onInputChange,
  disabled,
}: ProductFieldsProps) {
  const fields = requiredFieldNames(product)

  if (fields.length > 0) {
    return (
      <div className="space-y-3">
        {fields.map((name) => (
          <div key={name} className="space-y-1.5">
            <label htmlFor={`field-${name}`} className="text-[13px] font-medium text-ink">
              {fieldLabel(product, name)}
            </label>
            <Input
              id={`field-${name}`}
              value={inputs[name] ?? ""}
              onChange={(event) => onInputChange(name, event.target.value)}
              placeholder={fieldLabel(product, name)}
              autoComplete="off"
              disabled={disabled}
            />
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="space-y-1.5">
        <label htmlFor="serviceRef" className="text-[13px] font-medium text-ink">
          {product.service_ref_label}
        </label>
        <Input
          id="serviceRef"
          value={serviceRef}
          onChange={(event) => onServiceRefChange(event.target.value)}
          placeholder={product.service_ref_placeholder}
          autoComplete="off"
          disabled={disabled}
        />
      </div>
    </div>
  )
}
