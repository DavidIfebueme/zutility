"use client"

import * as React from "react"
import { motion } from "motion/react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./card"
import { Input } from "./input"
import { Button } from "./button"
import { toast } from "sonner"
import { cn } from "@/lib/utils"

interface ComingSoonOverlayProps extends React.HTMLAttributes<HTMLDivElement> {
  title: string
  subtitle: string
  feature: 'otc' | 'p2p'
  onNotify?: (email: string) => Promise<void>
}

export function ComingSoonOverlay({
  title,
  subtitle,
  feature,
  onNotify,
  className,
  ...props
}: ComingSoonOverlayProps) {
  const [email, setEmail] = React.useState("")
  const [loading, setLoading] = React.useState(false)
  const [success, setSuccess] = React.useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!email) return

    setLoading(true)
    try {
      if (onNotify) {
        await onNotify(email)
      } else {
        // Mock API call
        await new Promise(resolve => setTimeout(resolve, 1000))
      }
      setSuccess(true)
      toast.success("You're on the list!")
    } catch (error) {
      toast.error("Failed to join waitlist. Please try again.")
    } finally {
      setLoading(false)
    }
  }

  return (
    <div
      className={cn(
        "absolute inset-0 z-50 flex items-center justify-center bg-bg-void/80 backdrop-blur-sm",
        className
      )}
      {...props}
    >
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ duration: 0.4, ease: "easeOut" }}
        className="w-full max-w-md px-4"
      >
        <Card className="pointer-events-auto border-accent-zec/20 shadow-2xl">
          <CardHeader className="text-center">
            <CardTitle className="text-accent-zec mb-2">{title}</CardTitle>
            <CardDescription className="text-text-primary">
              {subtitle}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {success ? (
              <div className="rounded-lg bg-accent-green/10 p-4 text-center border border-accent-green/20">
                <p className="text-accent-green font-medium">You&apos;re on the list.</p>
                <p className="text-sm text-text-secondary mt-1">We&apos;ll notify you when it&apos;s ready.</p>
              </div>
            ) : (
              <form onSubmit={handleSubmit} className="flex flex-col gap-4">
                <Input
                  type="email"
                  placeholder="Enter your email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  required
                  className="bg-bg-void"
                />
                <Button type="submit" loading={loading} className="w-full">
                  Notify Me
                </Button>
              </form>
            )}
          </CardContent>
        </Card>
      </motion.div>
    </div>
  )
}
