import * as React from "react"
import { cn } from "@/lib/utils"
import { Check } from "lucide-react"

interface StepperProps extends React.HTMLAttributes<HTMLDivElement> {
  steps: { label: string }[]
  currentStep: number
}

export function Stepper({ steps, currentStep, className, ...props }: StepperProps) {
  return (
    <div className={cn("flex items-center justify-between w-full", className)} {...props}>
      {steps.map((step, index) => {
        const isCompleted = index < currentStep
        const isActive = index === currentStep

        return (
          <React.Fragment key={step.label}>
            <div className="flex flex-col items-center gap-2">
              <div
                className={cn(
                  "flex h-8 w-8 items-center justify-center rounded-full border-2 text-sm font-medium transition-colors",
                  isCompleted
                    ? "border-accent-zec bg-accent-zec text-bg-void"
                    : isActive
                    ? "border-accent-zec bg-bg-surface text-accent-zec"
                    : "border-border-subtle bg-bg-surface text-text-muted"
                )}
              >
                {isCompleted ? <Check className="h-4 w-4" /> : index + 1}
              </div>
              <span
                className={cn(
                  "text-xs font-medium hidden sm:block",
                  isCompleted || isActive ? "text-text-primary" : "text-text-muted"
                )}
              >
                {step.label}
              </span>
            </div>
            {index < steps.length - 1 && (
              <div
                className={cn(
                  "h-[2px] flex-1 mx-4 transition-colors",
                  isCompleted ? "bg-accent-zec" : "bg-border-subtle"
                )}
              />
            )}
          </React.Fragment>
        )
      })}
    </div>
  )
}
