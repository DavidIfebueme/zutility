import type { ComponentType } from "react"
import type { AnimatedIconProps } from "@/components/icons/types"
import type { BrandIconId } from "@/lib/constants"

import MtnIcon from "@/components/icons/brands/mtn-icon"
import AirtelIcon from "@/components/icons/brands/airtel-icon"
import GloIcon from "@/components/icons/brands/glo-icon"
import NineMobileIcon from "@/components/icons/brands/9mobile-icon"
import DstvIcon from "@/components/icons/brands/dstv-icon"
import GotvIcon from "@/components/icons/brands/gotv-icon"
import StartimesIcon from "@/components/icons/brands/startimes-icon"
import ShowmaxIcon from "@/components/icons/brands/showmax-icon"
import IkejaElectricIcon from "@/components/icons/brands/ikeja-electric-icon"
import EkoElectricIcon from "@/components/icons/brands/eko-electric-icon"
import AbujaElectricIcon from "@/components/icons/brands/abuja-electric-icon"
import IbadanElectricIcon from "@/components/icons/brands/ibadan-electric-icon"
import KanoElectricIcon from "@/components/icons/brands/kano-electric-icon"
import PhedElectricIcon from "@/components/icons/brands/phed-electric-icon"
import JosElectricIcon from "@/components/icons/brands/jos-electric-icon"
import KadunaElectricIcon from "@/components/icons/brands/kaduna-electric-icon"
import EnuguElectricIcon from "@/components/icons/brands/enugu-electric-icon"
import BeninElectricIcon from "@/components/icons/brands/benin-electric-icon"
import YolaElectricIcon from "@/components/icons/brands/yola-electric-icon"
import AbaElectricIcon from "@/components/icons/brands/aba-electric-icon"
import WaecIcon from "@/components/icons/brands/waec-icon"
import JambIcon from "@/components/icons/brands/jamb-icon"
import SchoolFeesIcon from "@/components/icons/brands/school-fees-icon"

const brandIconMap: Record<BrandIconId, ComponentType<AnimatedIconProps>> = {
  mtn: MtnIcon,
  airtel: AirtelIcon,
  glo: GloIcon,
  '9mobile': NineMobileIcon,
  dstv: DstvIcon,
  gotv: GotvIcon,
  startimes: StartimesIcon,
  showmax: ShowmaxIcon,
  'ikeja-electric': IkejaElectricIcon,
  'eko-electric': EkoElectricIcon,
  'abuja-electric': AbujaElectricIcon,
  'ibadan-electric': IbadanElectricIcon,
  'kano-electric': KanoElectricIcon,
  'phed-electric': PhedElectricIcon,
  'jos-electric': JosElectricIcon,
  'kaduna-electric': KadunaElectricIcon,
  'enugu-electric': EnuguElectricIcon,
  'benin-electric': BeninElectricIcon,
  'yola-electric': YolaElectricIcon,
  'aba-electric': AbaElectricIcon,
  waec: WaecIcon,
  jamb: JambIcon,
  'school-fees': SchoolFeesIcon,
}

export function getBrandIcon(id: BrandIconId): ComponentType<AnimatedIconProps> {
  return brandIconMap[id]
}
