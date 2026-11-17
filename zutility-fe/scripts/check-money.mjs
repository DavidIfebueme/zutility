import assert from 'node:assert/strict'
import { formatDisplay } from '../lib/money.ts'

const cases = [
  {
    name: 'XOF at zero decimals',
    display: { currency: 'XOF', amount: '1325', rate_available: true },
    naira: 0,
    text: '1\u202f325\u00a0CFA',
    marker: null,
  },
  {
    name: 'ZAR at two decimals',
    display: { currency: 'ZAR', amount: '1234.5', rate_available: true },
    naira: 0,
    text: 'R\u00a01\u00a0234,50',
    marker: null,
  },
  {
    name: 'USD at two decimals',
    display: { currency: 'USD', amount: '12.34', rate_available: true },
    naira: 0,
    text: '$12.34',
    marker: null,
  },
  {
    name: 'unavailable rate falls back to naira',
    display: { currency: 'KES', amount: null, rate_available: false },
    naira: 5000,
    text: '₦5,000.00',
    marker: 'NGN',
  },
]

let failed = 0

for (const item of cases) {
  const result = formatDisplay(item.display, item.naira)
  try {
    assert.equal(result.text, item.text, `${item.name} text`)
    assert.equal(result.marker, item.marker, `${item.name} marker`)
    console.log(`ok   ${item.name}: ${result.text}${result.marker ? ` · ${result.marker}` : ''}`)
  } catch (error) {
    failed += 1
    console.log(`fail ${item.name}: ${error instanceof Error ? error.message : String(error)}`)
    console.log(`     got ${JSON.stringify(result.text)} marker ${JSON.stringify(result.marker)}`)
  }
}

if (failed > 0) {
  console.log(`${failed} of ${cases.length} money cases failed`)
  process.exit(1)
}

console.log(`${cases.length} of ${cases.length} money cases passed`)
