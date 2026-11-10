import type { BrandIconId } from "@/lib/constants"

export const BRAND_LOGOS: Partial<Record<BrandIconId, string>> = {
  mtn: "/logos/mtn.svg",
  airtel: "/logos/airtel.svg",
  glo: "/logos/glo.png",
  "9mobile": "/logos/9mobile.png",
  dstv: "/logos/dstv.svg",
  gotv: "/logos/gotv.svg",
  startimes: "/logos/startimes.png",
  showmax: "/logos/showmax.png",
  "ikeja-electric": "/logos/ikeja-electric.png",
  "ibadan-electric": "/logos/ibadan-electric.png",
  "jos-electric": "/logos/jos-electric.png",
  "kaduna-electric": "/logos/kaduna-electric.png",
  "kano-electric": "/logos/kano-electric.png",
  "phed-electric": "/logos/phed-electric.png",
  "yola-electric": "/logos/yola-electric.png",
  waec: "/logos/waec.png",
  jamb: "/logos/jamb.png",
}

export const BRAND_MONOGRAMS: Partial<Record<BrandIconId, string>> = {
  "eko-electric": "EKEDC",
  "abuja-electric": "AEDC",
  "enugu-electric": "EEDC",
  "benin-electric": "BEDC",
  "aba-electric": "ABA",
  "school-fees": "SCH",
}

export const BRAND_TILE_BACKGROUND: Partial<Record<BrandIconId, string>> = {
  mtn: "#ffcb05",
}
