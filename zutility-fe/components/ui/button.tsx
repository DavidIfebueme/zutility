import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"
import { Loader2 } from "lucide-react"

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-bg-void transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-zec focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-accent-zec text-bg-void hover:bg-accent-zec-dim",
        primary: "bg-accent-zec text-bg-void hover:bg-accent-zec-dim",
        secondary: "bg-transparent text-text-primary border border-text-primary hover:bg-text-primary hover:text-bg-void",
        danger: "bg-accent-red text-white hover:bg-red-600",
        ghost: "hover:bg-bg-surface hover:text-text-primary",
        link: "text-accent-blue underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
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
        {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
        {loading && typeof children === 'string' ? 'Loading...' : children}
      </Comp>
    )
  }
)
Button.displayName = "Button"

export { Button, buttonVariants }
