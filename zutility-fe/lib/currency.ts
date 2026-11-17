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

export function detectCurrency(): CurrencyCode {
  if (typeof window === 'undefined') return DEFAULT_CURRENCY
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone
    return TIMEZONE_CURRENCY[tz] || DEFAULT_CURRENCY
  } catch {
    return DEFAULT_CURRENCY
  }
}

export function formatCurrency(amount: number | string, currency: CurrencyCode = 'NGN'): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount
  if (isNaN(num)) return `${CURRENCIES[currency].symbol}0.00`
  try {
    return new Intl.NumberFormat(CURRENCIES[currency].locale, {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(num)
  } catch {
    return `${CURRENCIES[currency].symbol}${num.toFixed(2)}`
  }
}

export interface FxRates {
  usd_ngn: number
  usd_kes: number
  usd_ghs: number
  usd_zar: number
  usd_egp: number
  usd_xof?: number
  usd_gmd?: number
  usd_mwk?: number
  usd_zmw?: number
}

function quotePerUsd(rate: RateResponse, code: string): number {
  const quote = (rate.fx ?? []).find((entry) => entry.quote === code)
  return quote ? parseFloat(quote.per_usd) || 0 : 0
}

export function fxRatesFromRate(rate: RateResponse): FxRates {
  return {
    usd_ngn: parseFloat(rate.usd_ngn) || 1,
    usd_kes: quotePerUsd(rate, 'KES') || parseFloat(rate.usd_kes) || 0,
    usd_ghs: quotePerUsd(rate, 'GHS') || parseFloat(rate.usd_ghs) || 0,
    usd_zar: quotePerUsd(rate, 'ZAR') || parseFloat(rate.usd_zar) || 0,
    usd_egp: quotePerUsd(rate, 'EGP') || parseFloat(rate.usd_egp) || 0,
    usd_xof: quotePerUsd(rate, 'XOF'),
    usd_gmd: quotePerUsd(rate, 'GMD'),
    usd_mwk: quotePerUsd(rate, 'MWK'),
    usd_zmw: quotePerUsd(rate, 'ZMW'),
  }
}

function perUsdFor(currency: CurrencyCode, rates: FxRates): number | null {
  switch (currency) {
    case 'KES': return rates.usd_kes || null
    case 'GHS': return rates.usd_ghs || null
    case 'ZAR': return rates.usd_zar || null
    case 'EGP': return rates.usd_egp || null
    case 'XOF': return rates.usd_xof || null
    case 'GMD': return rates.usd_gmd || null
    case 'MWK': return rates.usd_mwk || null
    case 'ZMW': return rates.usd_zmw || null
    default: return null
  }
}

export function convertFromNGN(amountNGN: number, targetCurrency: CurrencyCode, rates: FxRates): number {
  if (targetCurrency === 'NGN') return amountNGN
  const usdAmount = amountNGN / (rates.usd_ngn || 1)
  if (targetCurrency === 'USD') return usdAmount
  const perUsd = perUsdFor(targetCurrency, rates)
  if (perUsd === null) return 0
  return usdAmount * perUsd
}

export function convertToNGN(amountLocal: number, fromCurrency: CurrencyCode, rates: FxRates): number {
  if (fromCurrency === 'NGN') return amountLocal
  if (fromCurrency === 'USD') return amountLocal * (rates.usd_ngn || 1)
  const perUsd = perUsdFor(fromCurrency, rates)
  if (perUsd === null) return amountLocal
  return (amountLocal / perUsd) * (rates.usd_ngn || 1)
}

export function formatLocalAmount(amountNGN: number, currency: CurrencyCode, rates: FxRates): string {
  if (currency === 'NGN') return formatCurrency(amountNGN, 'NGN')
  const converted = convertFromNGN(amountNGN, currency, rates)
  if (converted === 0) return formatCurrency(amountNGN, 'NGN')
  return formatCurrency(converted, currency)
}
