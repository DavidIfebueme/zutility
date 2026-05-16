"use client"

import * as React from "react"
import Link from "next/link"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import { Mail, ArrowLeft, ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const forgotSchema = z.object({
  email: z.string().email("Invalid email address"),
})

type ForgotFormValues = z.infer<typeof forgotSchema>

export default function ForgotPasswordPage() {
  const [isLoading, setIsLoading] = React.useState(false)
  const [sent, setSent] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<ForgotFormValues>({
    resolver: zodResolver(forgotSchema),
  })

  const onSubmit = async (data: ForgotFormValues) => {
    setIsLoading(true)
    try {
      const res = await apiPostRaw("/api/v1/auth/forgot-password", { email: data.email })
      setSent(true)
      toast.success("If an account exists with that email, a reset link has been sent.")
    } catch {
      setSent(true)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      <div className="hidden w-1/2 flex-col justify-between border-r border-border-subtle bg-bg-surface p-12 lg:flex relative overflow-hidden">
        <div className="absolute inset-0 z-0 opacity-30 pointer-events-none">
          <Canvas camera={{ position: [0, 0, 5], fov: 45 }}>
            <ambientLight intensity={0.5} />
            <ZecCoinScene />
          </Canvas>
        </div>
        <div className="relative z-10">
          <Link href="/" className="font-dela text-2xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
        </div>
        <div className="relative z-10 max-w-md">
          <h1 className="font-dela text-4xl leading-tight mb-4">
            Secure your account.
          </h1>
          <p className="text-text-secondary text-lg">
            Reset your password and get back to managing your utility payments privately.
          </p>
        </div>
      </div>

      <div className="flex w-full flex-col justify-center px-8 sm:px-16 lg:w-1/2 xl:px-24">
        <div className="mx-auto w-full max-w-sm">
          <div className="mb-10 lg:hidden">
            <Link href="/" className="font-dela text-2xl tracking-tight">
              <span className="text-accent-zec">z</span>utility
            </Link>
          </div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
          >
            <h2 className="font-dela text-3xl mb-2">Forgot Password</h2>
            <p className="text-text-secondary mb-8">
              Remember your password?{" "}
              <Link href="/login" className="text-accent-zec hover:underline">
                Log in
              </Link>
            </p>

            {sent ? (
              <div className="rounded-lg border border-border-subtle bg-bg-elevated p-6 text-center">
                <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
                  <Mail className="h-6 w-6" />
                </div>
                <h3 className="font-semibold mb-2">Check your email</h3>
                <p className="text-sm text-text-secondary">
                  If an account exists with that email, you&apos;ll receive a password reset link shortly.
                </p>
              </div>
            ) : (
              <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
                <div className="space-y-1">
                  <Input
                    {...register("email")}
                    type="email"
                    placeholder="Email address"
                    leftIcon={<Mail className="h-5 w-5" />}
                    error={errors.email?.message}
                  />
                </div>

                <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                  Send Reset Link <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </form>
            )}

            <div className="mt-8">
              <Link href="/login" className="inline-flex items-center gap-2 text-sm text-text-secondary hover:text-text-primary transition-colors">
                <ArrowLeft className="h-4 w-4" />
                Back to login
              </Link>
            </div>
          </motion.div>
        </div>
      </div>
    </div>
  )
}
