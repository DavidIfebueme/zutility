import type { ComponentType } from "react"
import type { AnimatedIconProps } from "@/components/icons/types"
import type { UtilityType } from "@/lib/constants"

import PhoneVolume from "@/components/icons/phone-volume"
import WifiIcon from "@/components/icons/wifi-icon"
import TvIcon from "@/components/icons/tv-icon"
import PlugConnectedIcon from "@/components/icons/plug-connected-icon"
import BookIcon from "@/components/icons/book-icon"
import SchoolFeesIcon from "@/components/icons/brands/school-fees-icon"

const categoryIconMap: Record<UtilityType | 'school', ComponentType<AnimatedIconProps>> = {
  airtime: PhoneVolume,
  data: WifiIcon,
  tv: TvIcon,
  electricity: PlugConnectedIcon,
  education: BookIcon,
  school: SchoolFeesIcon,
}

export function getCategoryIcon(type: UtilityType | 'school'): ComponentType<AnimatedIconProps> {
  return categoryIconMap[type]
}
