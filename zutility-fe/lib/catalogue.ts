import type {
  CatalogueCategory,
  CatalogueCountry,
  CatalogueProduct,
  CatalogueResponse,
  CatalogueVariationItem,
} from '@/lib/types'

export const CATEGORY_ORDER = [
  'airtime',
  'data',
  'television',
  'electricity',
  'education',
  'school',
] as const

export const CATEGORY_LABELS: Record<string, string> = {
  airtime: 'Airtime',
  data: 'Data',
  television: 'TV',
  electricity: 'Electricity',
  education: 'Education',
  school: 'School fees',
  internet: 'Internet',
  landline: 'Landline',
  voip: 'VoIP',
}

const CATEGORY_UTILITY_TYPE: Record<string, string> = {
  airtime: 'airtime',
  data: 'data',
  television: 'television',
  electricity: 'electricity',
  education: 'waec',
  school: 'school_fees',
  internet: 'internet',
  landline: 'landline',
  voip: 'voip',
}

export function categoryLabel(category: string): string {
  return CATEGORY_LABELS[category] ?? category
}

export function categoryUtilityType(category: string): string {
  return CATEGORY_UTILITY_TYPE[category] ?? 'airtime'
}

export function countriesWithProducts(catalogue: CatalogueResponse | null): CatalogueCountry[] {
  if (!catalogue) return []
  return catalogue.countries.filter((country) =>
    country.categories.some((category) => category.products.length > 0)
  )
}

export function categoriesWithProducts(country: CatalogueCountry | null): CatalogueCategory[] {
  if (!country) return []
  const orderOf = (category: string) => {
    const index = CATEGORY_ORDER.indexOf(category as (typeof CATEGORY_ORDER)[number])
    return index === -1 ? CATEGORY_ORDER.length : index
  }
  return [...country.categories]
    .filter((category) => category.products.length > 0)
    .sort((left, right) => orderOf(left.category) - orderOf(right.category))
}

export interface CatalogueSelection {
  product: CatalogueProduct
  category: string
}

export function findProduct(
  country: CatalogueCountry | null,
  productId: string | null
): CatalogueSelection | null {
  if (!country || !productId) return null
  for (const category of country.categories) {
    const product = category.products.find((item) => item.id === productId)
    if (product) return { product, category: category.category }
  }
  return null
}

export function findProductBySlug(
  catalogue: CatalogueResponse | null,
  slug: string
): CatalogueProduct | null {
  if (!catalogue) return null
  for (const country of catalogue.countries) {
    for (const category of country.categories) {
      const product = category.products.find((item) => item.slug === slug)
      if (product) return product
    }
  }
  return null
}

export function requiredFieldNames(product: CatalogueProduct): string[] {
  const fields = product.required_fields
  const names = [
    ...(fields.credit_party_identifier ?? []),
    ...(fields.beneficiary ?? []),
    ...(fields.additional_identifier ?? []),
  ]
  return [...new Set(names)]
}

export function fieldLabel(product: CatalogueProduct, name: string): string {
  const configured = product.field_labels[name]
  if (configured && configured.trim()) return configured
  const words = name.replace(/[_-]+/g, ' ').trim()
  return words.charAt(0).toUpperCase() + words.slice(1)
}

export function isPhoneField(product: CatalogueProduct): boolean {
  const label = product.service_ref_label.toLowerCase()
  return label.includes('phone') || label.includes('mobile')
}

export interface CatalogueBiller {
  name: string
  serviceRefLabel: string
  serviceRefPlaceholder: string
  isPhone: boolean
}

export function billerFromProduct(product: CatalogueProduct | null): CatalogueBiller | null {
  if (!product) return null
  return {
    name: product.name,
    serviceRefLabel: product.service_ref_label,
    serviceRefPlaceholder: product.service_ref_placeholder,
    isPhone: isPhoneField(product),
  }
}

export function billerBySlug(
  catalogue: CatalogueResponse | null,
  slug: string
): CatalogueBiller | null {
  return billerFromProduct(findProductBySlug(catalogue, slug))
}

export interface LiveVariationRow {
  variation_code: string
  name: string
  amount: number | null
}

export function needsLiveVariations(product: CatalogueProduct | null): boolean {
  return Boolean(product && product.amount_kind === 'variation' && product.variations.length === 0)
}

export function variationsFromLive(rows: unknown): CatalogueVariationItem[] {
  if (!Array.isArray(rows)) return []
  const seen = new Set<string>()
  const items: CatalogueVariationItem[] = []
  for (const row of rows as LiveVariationRow[]) {
    const code = typeof row?.variation_code === 'string' ? row.variation_code.trim() : ''
    if (!code || seen.has(code)) continue
    const amount = Number(row?.amount)
    if (!Number.isFinite(amount) || amount <= 0) continue
    seen.add(code)
    items.push({
      variation_code: code,
      name: typeof row?.name === 'string' && row.name.trim() ? row.name.trim() : code,
      amount: String(amount),
    })
  }
  return items
}

export interface AmountSelection {
  amount: string | null
  error: string | null
}

export function resolveAmount(
  product: CatalogueProduct | null,
  variationCode: string | null,
  amountInput: string
): AmountSelection {
  if (!product) return { amount: null, error: 'Select a service first' }

  if (product.amount_kind === 'variation') {
    if (!variationCode) return { amount: null, error: 'Choose a plan' }
    const variation = product.variations.find((item) => item.variation_code === variationCode)
    if (!variation) return { amount: null, error: 'Choose a plan' }
    return { amount: variation.amount, error: null }
  }

  if (product.amount_kind === 'fixed') {
    if (!product.amount_min) return { amount: null, error: 'This service has no price yet' }
    return { amount: product.amount_min, error: null }
  }

  const amount = Number(amountInput)
  if (!Number.isFinite(amount) || amount <= 0) {
    return { amount: null, error: 'Enter an amount' }
  }
  const minimum = Number(product.amount_min ?? 0)
  const maximum = Number(product.amount_max ?? 0)
  if (amount < minimum) {
    return { amount: null, error: `Minimum is ${product.amount_min} ${product.currency}` }
  }
  if (maximum > 0 && amount > maximum) {
    return { amount: null, error: `Maximum is ${product.amount_max} ${product.currency}` }
  }
  const increment = Number(product.amount_increment ?? 0)
  if (increment > 0) {
    const steps = (amount - minimum) / increment
    if (Math.abs(steps - Math.round(steps)) > 1e-6) {
      return {
        amount: null,
        error: `Amounts step by ${product.amount_increment} ${product.currency}`,
      }
    }
  }
  return { amount: amountInput.trim(), error: null }
}

export function quickAmounts(product: CatalogueProduct): number[] {
  return Array.isArray(product.quick_amounts) ? product.quick_amounts : []
}
