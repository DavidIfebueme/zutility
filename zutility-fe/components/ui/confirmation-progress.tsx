"use client"

import * as React from "react"
import { motion } from "motion/react"
import { cn } from "@/lib/utils"

interface ConfirmationProgressProps extends React.HTMLAttributes<HTMLDivElement> {
  current: number
  required: number
  addressType: 'shielded' | 'transparent'
}

export function ConfirmationProgress({
  current,
  required,
  addressType,
  className,
  ...props
}: ConfirmationProgressProps) {
  const progress = Math.min(Math.max(current / required, 0), 1)
  const isComplete = current >= required

  return (
    <div className={cn("relative flex flex-col items-center justify-center p-8", className)} {...props}>
      <svg className="w-48 h-48 transform -rotate-90" viewBox="0 0 100 100">
        {/* Background track */}
        <circle
          cx="50"
          cy="50"
          r="45"
          fill="transparent"
          stroke="var(--color-border-subtle)"
          strokeWidth="6"
        />
        
        {/* Progress arc */}
        <motion.circle
          cx="50"
          cy="50"
          r="45"
          fill="transparent"
          stroke={isComplete ? "var(--color-accent-green)" : "var(--color-accent-zec)"}
          strokeWidth="6"
          strokeDasharray="283"
          strokeDashoffset={283 - 283 * progress}
          strokeLinecap="round"
          initial={{ strokeDashoffset: 283 }}
          animate={{ strokeDashoffset: 283 - 283 * progress }}
          transition={{ duration: 1, ease: "easeInOut" }}
        />

        {/* Checkmark animation on complete */}
        {isComplete && (
          <motion.path
            d="M 35 50 L 45 60 L 65 40"
            fill="transparent"
            stroke="var(--color-accent-green)"
            strokeWidth="6"
            strokeLinecap="round"
            strokeLinejoin="round"
            initial={{ pathLength: 0 }}
            animate={{ pathLength: 1 }}
            transition={{ duration: 0.5, delay: 0.5 }}
          />
        )}
      </svg>

      {!isComplete && (
        <div className="absolute inset-0 flex flex-col items-center justify-center text-center">
          <span className="font-mono text-3xl font-bold text-accent-zec">
            {current}
            <span className="text-text-muted">/{required}</span>
          </span>
          <span className="text-xs text-text-secondary mt-1 uppercase tracking-wider">
            Confirmations
          </span>
        </div>
      )}
    </div>
  )
}
