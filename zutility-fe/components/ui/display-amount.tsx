import { formatDisplay, type DisplayMoney } from "@/lib/money"

interface DisplayAmountProps {
  display: DisplayMoney
  nairaAmount: number
  className?: string
}

export function DisplayAmount({ display, nairaAmount, className }: DisplayAmountProps) {
  const { text, marker } = formatDisplay(display, nairaAmount)

  return (
    <span className={className}>
      {text}
      {marker ? <span className="ml-1 text-[11px] text-muted">{marker}</span> : null}
    </span>
  )
}
