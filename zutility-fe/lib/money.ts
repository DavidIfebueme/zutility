export const CURRENCY_DISPLAY = {
  NGN: { symbol: '₦', locale: 'en-NG', name: 'Naira', country: 'Nigeria' },
  XOF: { symbol: 'CFA', locale: 'fr-SN', name: 'West African CFA franc', country: 'West Africa' },
  GMD: { symbol: 'D', locale: 'en-GM', name: 'Dalasi', country: 'Gambia' },
  MWK: { symbol: 'MK', locale: 'en-MW', name: 'Kwacha', country: 'Malawi' },
  ZMW: { symbol: 'K', locale: 'en-ZM', name: 'Zambian kwacha', country: 'Zambia' },
  ZAR: { symbol: 'R', locale: 'en-ZA', name: 'Rand', country: 'South Africa' },
  USD: { symbol: '$', locale: 'en-US', name: 'US dollar', country: 'Global' },
  KES: { symbol: 'KSh', locale: 'en-KE', name: 'Kenyan shilling', country: 'Kenya' },
  GHS: { symbol: 'GH₵', locale: 'en-GH', name: 'Cedi', country: 'Ghana' },
  EGP: { symbol: 'E£', locale: 'ar-EG', name: 'Egyptian pound', country: 'Egypt' },
} as const

export type CurrencyCode = keyof typeof CURRENCY_DISPLAY

export interface DisplayMoney {
  currency: string
  amount: string | null
  rate_available: boolean
}

export interface DisplayText {
  text: string
  marker: string | null
}

export function isCurrencyCode(value: string): value is CurrencyCode {
  return Object.prototype.hasOwnProperty.call(CURRENCY_DISPLAY, value)
}

export function formatAmount(value: number, currency: CurrencyCode): string {
  const { symbol, locale } = CURRENCY_DISPLAY[currency]
  const parts = new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
  }).formatToParts(value)
  return parts.map((part) => (part.type === 'currency' ? symbol : part.value)).join('')
}

export function formatDisplay(display: DisplayMoney, nairaAmount: number): DisplayText {
  const currency = isCurrencyCode(display.currency) ? display.currency : 'NGN'
  const amount = display.amount === null ? Number.NaN : Number(display.amount)

  if (display.rate_available && Number.isFinite(amount)) {
    return { text: formatAmount(amount, currency), marker: null }
  }

  return { text: formatAmount(nairaAmount, 'NGN'), marker: 'NGN' }
}
