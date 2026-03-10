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
          <div className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted">
            {leftIcon}
          </div>
        )}
        <input
          type={type}
          className={cn(
            "flex h-12 w-full rounded-md border border-border-subtle bg-bg-elevated px-4 py-2 text-sm text-text-primary ring-offset-bg-void file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-zec focus-visible:border-transparent disabled:cursor-not-allowed disabled:opacity-50 transition-colors",
            leftIcon && "pl-10",
            rightElement && "pr-12",
            error && "border-accent-red focus-visible:ring-accent-red",
            className
          )}
          ref={ref}
          {...props}
        />
        {rightElement && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2">
            {rightElement}
          </div>
        )}
        {error && (
          <p className="mt-1.5 text-xs text-accent-red">{error}</p>
        )}
      </div>
    )
  }
)
Input.displayName = "Input"

export { Input }
