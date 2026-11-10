import * as React from "react"
import { Check } from "lucide-react"
import { cn } from "@/lib/utils"

interface StepperProps extends React.HTMLAttributes<HTMLDivElement> {
  steps: { label: string }[]
  currentStep: number
}

export function Stepper({ steps, currentStep, className, ...props }: StepperProps) {
  return (
    <div className={cn("flex items-center gap-2", className)} {...props}>
      {steps.map((step, index) => {
        const isDone = index < currentStep
        const isActive = index === currentStep

        return (
          <React.Fragment key={step.label}>
            <div className="flex min-w-0 items-center gap-1.5">
              <span
                className={cn(
                  "flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[10px] font-medium",
                  isDone
                    ? "bg-green text-cream"
                    : isActive
                      ? "bg-ink text-cream"
                      : "border border-line text-muted"
                )}
              >
                {isDone ? <Check className="h-3 w-3" /> : index + 1}
              </span>
              <span
                className={cn(
                  "hidden truncate text-[12px] sm:block",
                  isActive || isDone ? "text-ink" : "text-muted"
                )}
              >
                {step.label}
              </span>
            </div>
            {index < steps.length - 1 && <span className="h-px flex-1 bg-line" />}
          </React.Fragment>
        )
      })}
    </div>
  )
}
