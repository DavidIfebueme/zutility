import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"
import { Loader2 } from "lucide-react"

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors duration-150 focus-visible:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gold-deep disabled:pointer-events-none disabled:opacity-45",
  {
    variants: {
      variant: {
        default:
          "bg-ink text-cream hover:bg-ink-2 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100",
        primary:
          "bg-ink text-cream hover:bg-ink-2 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100",
        accent:
          "bg-gold text-ink hover:bg-gold/85 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100",
        secondary:
          "border border-line-strong bg-cream text-ink hover:bg-paper-2 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100",
        danger: "bg-rust text-cream hover:bg-rust/90 disabled:opacity-100 disabled:bg-paper-3 disabled:text-muted",
        ghost: "text-ink hover:bg-paper-3/60",
        link: "text-gold-deep underline underline-offset-4 hover:no-underline",
      },
      size: {
        default: "h-10 px-4",
        sm: "h-9 px-3 text-[13px]",
        lg: "h-12 px-6 text-[15px]",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean
  loading?: boolean
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, loading, children, ...props }, ref) => {
    const Comp = asChild ? Slot : "button"
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        disabled={loading || props.disabled}
        {...props}
      >
        {loading ? (
          <>
            <Loader2 className="h-4 w-4 animate-spin" />
            {typeof children === 'string' ? 'Working' : children}
          </>
        ) : (
          children
        )}
      </Comp>
    )
  }
)
Button.displayName = "Button"

export { Button, buttonVariants }
