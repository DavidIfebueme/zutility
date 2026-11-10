import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"
import { Loader2 } from "lucide-react"

const buttonVariants = cva(
  "group/btn inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-[color,background-color,border-color,box-shadow,transform] duration-150 ease-out focus-visible:outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gold-deep active:translate-y-px disabled:pointer-events-none disabled:opacity-45 motion-reduce:transform-none",
  {
    variants: {
      variant: {
        default:
          "bg-ink text-cream shadow-[0_1px_0_0_var(--line-strong)] hover:-translate-y-0.5 hover:bg-ink-2 hover:shadow-[0_3px_0_0_var(--line-strong)] active:translate-y-0 active:shadow-[0_1px_0_0_var(--line-strong)] disabled:translate-y-0 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100 disabled:shadow-none",
        primary:
          "bg-ink text-cream shadow-[0_1px_0_0_var(--line-strong)] hover:-translate-y-0.5 hover:bg-ink-2 hover:shadow-[0_3px_0_0_var(--line-strong)] active:translate-y-0 active:shadow-[0_1px_0_0_var(--line-strong)] disabled:translate-y-0 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100 disabled:shadow-none",
        accent:
          "bg-gold text-ink shadow-[0_1px_0_0_var(--gold-deep)] hover:-translate-y-0.5 hover:bg-gold/90 hover:shadow-[0_3px_0_0_var(--gold-deep)] active:translate-y-0 active:shadow-[0_1px_0_0_var(--gold-deep)] disabled:translate-y-0 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100 disabled:shadow-none",
        secondary:
          "border border-line-strong bg-cream text-ink shadow-[0_1px_0_0_var(--line)] hover:-translate-y-0.5 hover:bg-paper-2 hover:shadow-[0_3px_0_0_var(--line)] active:translate-y-0 active:shadow-[0_1px_0_0_var(--line)] disabled:translate-y-0 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100 disabled:shadow-none",
        danger:
          "bg-rust text-cream shadow-[0_1px_0_0_color-mix(in_srgb,var(--rust)_60%,black)] hover:-translate-y-0.5 hover:bg-rust/90 active:translate-y-0 disabled:translate-y-0 disabled:bg-paper-3 disabled:text-muted disabled:opacity-100 disabled:shadow-none",
        ghost: "text-ink hover:bg-paper-3/60",
        link: "text-gold-deep u-link",
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
