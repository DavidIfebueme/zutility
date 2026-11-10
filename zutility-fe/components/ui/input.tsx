import * as React from "react"
import { cn } from "@/lib/utils"

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {
  error?: string
  leftIcon?: React.ReactNode
  rightElement?: React.ReactNode
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, error, leftIcon, rightElement, ...props }, ref) => {
    return (
      <div className="relative w-full">
        {leftIcon && (
          <div className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted">
            {leftIcon}
          </div>
        )}
        <input
          type={type}
          className={cn(
            "flex h-11 w-full rounded-lg border border-line bg-cream px-3 py-2 text-sm text-ink placeholder:text-muted/70 transition-colors focus-visible:border-line-strong focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-gold-deep disabled:cursor-not-allowed disabled:bg-paper-2 disabled:opacity-60",
            leftIcon && "pl-10",
            rightElement && "pr-12",
            error && "border-rust focus-visible:outline-rust",
            className
          )}
          ref={ref}
          {...props}
        />
        {rightElement && (
          <div className="absolute right-2 top-1/2 -translate-y-1/2">
            {rightElement}
          </div>
        )}
        {error && (
          <p className="mt-1.5 text-xs text-rust">{error}</p>
        )}
      </div>
    )
  }
)
Input.displayName = "Input"

export { Input }
