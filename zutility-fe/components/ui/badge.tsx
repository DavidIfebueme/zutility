import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

const badgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium tracking-wide",
  {
    variants: {
      variant: {
        default: "border-line bg-paper-2 text-ink-2",
        live: "border-green/25 bg-green/8 text-green",
        "coming-soon": "border-line bg-transparent text-muted",
        success: "border-green/25 bg-green/8 text-green",
        warning: "border-gold-deep/25 bg-gold/15 text-gold-deep",
        error: "border-rust/25 bg-rust/8 text-rust",
        outline: "border-line-strong bg-transparent text-ink-2",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props}>
      {variant === 'live' && (
        <span className="h-1.5 w-1.5 rounded-full bg-green" aria-hidden="true" />
      )}
      {props.children}
    </div>
  )
}

export { Badge, badgeVariants }
