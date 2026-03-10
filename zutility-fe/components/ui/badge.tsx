import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

const badgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
  {
    variants: {
      variant: {
        default:
          "border-transparent bg-bg-elevated text-text-primary hover:bg-bg-elevated/80",
        live: "border-transparent bg-accent-green/10 text-accent-green hover:bg-accent-green/20",
        "coming-soon":
          "border-transparent bg-accent-zec/10 text-accent-zec hover:bg-accent-zec/20",
        success:
          "border-transparent bg-accent-green/10 text-accent-green hover:bg-accent-green/20",
        warning:
          "border-transparent bg-accent-zec/10 text-accent-zec hover:bg-accent-zec/20",
        error:
          "border-transparent bg-accent-red/10 text-accent-red hover:bg-accent-red/20",
        outline: "text-text-primary border-border-subtle",
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
        <span className="mr-1.5 flex h-2 w-2 relative">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-accent-green opacity-75"></span>
          <span className="relative inline-flex rounded-full h-2 w-2 bg-accent-green"></span>
        </span>
      )}
      {props.children}
    </div>
  )
}

export { Badge, badgeVariants }
