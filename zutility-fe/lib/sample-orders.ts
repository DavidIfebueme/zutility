import type { BrandIconId } from '@/lib/constants'

export interface SampleOrder {
  id: string
  utility: string
  detail: string
  amountNgn: number
  zec: string
  brand: BrandIconId
}

export const SAMPLE_ORDERS: SampleOrder[] = [
  {
    id: "8F2C-41A9",
    utility: "MTN Airtime",
    detail: "Phone ****",
    amountNgn: 2000,
    zec: "0.00153524",
    brand: "mtn",
  },
  {
    id: "2B7D-90C4",
    utility: "Ikeja Electric",
    detail: "Meter ****",
    amountNgn: 10000,
    zec: "0.00767618",
    brand: "ikeja-electric",
  },
  {
    id: "5A1E-73B8",
    utility: "School Fees",
    detail: "UNN · RRR ****",
    amountNgn: 52000,
    zec: "0.03991616",
    brand: "unn",
  },
  {
    id: "C3F6-2D10",
    utility: "DStv Compact",
    detail: "Smartcard ****",
    amountNgn: 19000,
    zec: "0.01458475",
    brand: "dstv",
  },
  {
    id: "9D4B-55E2",
    utility: "Airtel Data",
    detail: "10GB · Phone ****",
    amountNgn: 5000,
    zec: "0.00383809",
    brand: "airtel",
  },
]
