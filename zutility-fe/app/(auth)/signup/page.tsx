"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import MailFilledIcon from "@/components/icons/mail-filled-icon"
import LockIcon from "@/components/icons/lock-icon"
import UserIcon from "@/components/icons/user-icon"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { apiPostRaw } from "@/lib/api"
import { toast } from "sonner"
import { PaymentFlow } from "@/components/ui/payment-flow"

const signupSchema = z.object({
  email: z.string().email("Invalid email address"),
  display_name: z.string().min(1, "Display name is required"),
  password: z.string().min(8, "Password must be at least 8 characters"),
  confirm_password: z.string().min(1, "Please confirm your password"),
}).refine((data) => data.password === data.confirm_password, {
  message: "Passwords do not match",
  path: ["confirm_password"],
})

type SignupFormValues = z.infer<typeof signupSchema>

export default function SignupPage() {
  const router = useRouter()
  const [isLoading, setIsLoading] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<SignupFormValues>({
    resolver: zodResolver(signupSchema),
  })

  const onSubmit = async (data: SignupFormValues) => {
    setIsLoading(true)
    try {
      const res = await apiPostRaw("/api/v1/auth/register", {
        email: data.email,
        password: data.password,
        display_name: data.display_name,
      })
      if (res.ok) {
        toast.success("Account created! Please verify your email.")
        router.push(`/verify?email=${encodeURIComponent(data.email)}`)
      } else {
        const result = await res.json().catch(() => ({ error: "Registration failed" }))
        toast.error(result.error || "Could not create account")
      }
    } catch {
      toast.error("An error occurred during registration.")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      <div className="hidden w-1/2 flex-col justify-between border-r border-border-subtle bg-bg-surface p-12 lg:flex relative overflow-hidden">
        <div className="absolute inset-0 z-0 opacity-30 pointer-events-none">
          <PaymentFlow />
        </div>
        <div className="relative z-10">
          <Link href="/" className="font-dela text-2xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
        </div>
        <div className="relative z-10 max-w-md">
          <h1 className="font-dela text-4xl leading-tight mb-4">
            Join the private economy.
          </h1>
          <p className="text-text-secondary text-lg">
            Pay utilities with Zcash. No KYC. No middlemen. Just you and your tokens.
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
            <h2 className="font-dela text-3xl mb-2">Create Account</h2>
            <p className="text-text-secondary mb-8">
              Already have an account?{" "}
              <Link href="/login" className="text-accent-zec hover:underline">
                Log in
              </Link>
            </p>

            <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
              <div className="space-y-1">
                <Input
                  {...register("email")}
                  type="email"
                  placeholder="Email address"
                  leftIcon={<MailFilledIcon size={20} />}
                  error={errors.email?.message}
                />
              </div>

              <div className="space-y-1">
                <Input
                  {...register("display_name")}
                  type="text"
                  placeholder="Display name"
                  leftIcon={<UserIcon size={20} />}
                  error={errors.display_name?.message}
                />
              </div>

              <div className="space-y-1">
                <Input
                  {...register("password")}
                  type="password"
                  placeholder="Password"
                  leftIcon={<LockIcon size={20} />}
                  error={errors.password?.message}
                />
              </div>

              <div className="space-y-1">
                <Input
                  {...register("confirm_password")}
                  type="password"
                  placeholder="Confirm password"
                  leftIcon={<LockIcon size={20} />}
                  error={errors.confirm_password?.message}
                />
              </div>

              <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                Sign Up <ArrowNarrowRightIcon size={16} className="ml-2" />
              </Button>
            </form>
          </motion.div>
        </div>
      </div>
    </div>
  )
}
