import { CURRENCY_DISPLAY, isCurrencyCode, type CurrencyCode } from '@/lib/money'
import type { RateResponse } from '@/lib/types'

export type { CurrencyCode } from '@/lib/money'
export { isCurrencyCode, formatAmount, formatDisplay } from '@/lib/money'

export interface CurrencyInfo {
  code: CurrencyCode
  symbol: string
  locale: string
  name: string
  country: string
}

export const CURRENCIES = Object.fromEntries(
  Object.entries(CURRENCY_DISPLAY).map(([code, info]) => [code, { code, ...info }])
) as Record<CurrencyCode, CurrencyInfo>

export const DEFAULT_CURRENCY: CurrencyCode = 'NGN'
export const CURRENCY_COOKIE = 'display_currency'

const COUNTRY_CURRENCY: Record<string, CurrencyCode> = {
  NG: 'NGN',
  SN: 'XOF',
  ML: 'XOF',
  BJ: 'XOF',
  BF: 'XOF',
  CI: 'XOF',
  TG: 'XOF',
  GM: 'GMD',
  MW: 'MWK',
  ZW: 'USD',
  ZM: 'ZMW',
  ZA: 'ZAR',
  KE: 'KES',
  GH: 'GHS',
  EG: 'EGP',
}

const TIMEZONE_CURRENCY: Record<string, CurrencyCode> = {
  'Africa/Lagos': 'NGN',
  'Africa/Porto-Novo': 'XOF',
  'Africa/Niamey': 'XOF',
  'Africa/Bissau': 'XOF',
  'Africa/Dakar': 'XOF',
  'Africa/Bamako': 'XOF',
  'Africa/Ouagadougou': 'XOF',
  'Africa/Abidjan': 'XOF',
  'Africa/Lome': 'XOF',
  'Africa/Banjul': 'GMD',
  'Africa/Nairobi': 'KES',
  'Africa/Addis_Ababa': 'KES',
  'Africa/Dar_es_Salaam': 'KES',
  'Africa/Kampala': 'KES',
  'Africa/Mogadishu': 'KES',
  'Africa/Asmara': 'KES',
  'Africa/Accra': 'GHS',
  'Africa/Johannesburg': 'ZAR',
  'Africa/Maseru': 'ZAR',
  'Africa/Mbabane': 'ZAR',
  'Africa/Maputo': 'ZAR',
  'Africa/Gaborone': 'ZAR',
  'Africa/Windhoek': 'ZAR',
  'Africa/Harare': 'USD',
  'Africa/Lusaka': 'ZMW',
  'Africa/Blantyre': 'MWK',
  'Africa/Cairo': 'EGP',
  'Africa/Tripoli': 'EGP',
  'Africa/Khartoum': 'EGP',
  'Africa/Tunis': 'EGP',
  'Africa/Algiers': 'EGP',
  'Africa/Casablanca': 'EGP',
}

export function currencyFromCountry(iso2: string): CurrencyCode | null {
  return COUNTRY_CURRENCY[iso2.toUpperCase()] ?? null
}

export function readCurrencyCookie(): CurrencyCode | null {
  if (typeof document === 'undefined') return null
  const entry = document.cookie
    .split('; ')
    .find((row) => row.startsWith(`${CURRENCY_COOKIE}=`))
  if (!entry) return null
  const value = decodeURIComponent(entry.slice(CURRENCY_COOKIE.length + 1))
  return isCurrencyCode(value) ? value : null
}

export function persistDisplayCurrency(code: CurrencyCode) {
  if (typeof document === 'undefined') return
  document.cookie = `${CURRENCY_COOKIE}=${code}; path=/; max-age=${60 * 60 * 24 * 365}; samesite=lax`
}

export function detectCurrency(): CurrencyCode {
  if (typeof window === 'undefined') return DEFAULT_CURRENCY
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
    return TIMEZONE_CURRENCY[tz] || DEFAULT_CURRENCY
  } catch {
    return DEFAULT_CURRENCY
  }
}

export function zecPriceInCurrency(rate: RateResponse, currency: CurrencyCode): number | null {
  if (currency === 'NGN') return parseFloat(rate.zec_ngn) || null
  if (currency === 'USD') return parseFloat(rate.zec_usd) || null
  const usdNgn = parseFloat(rate.usd_ngn) || 0
  const quote = (rate.fx ?? []).find((entry) => entry.quote === currency)
  const perUsd = quote ? parseFloat(quote.per_usd) || 0 : 0
  if (usdNgn <= 0 || perUsd <= 0) return null
  return (parseFloat(rate.zec_ngn) / usdNgn) * perUsd
}
