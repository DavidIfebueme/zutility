"use client"

import * as React from "react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import { Mail, Lock, User, ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const signupSchema = z.object({
  email: z.string().email("Invalid email address"),
  displayName: z.string().optional(),
  password: z.string().min(8, "Password must be at least 8 characters"),
  confirmPassword: z.string()
}).refine((data) => data.password === data.confirmPassword, {
  message: "Passwords don't match",
  path: ["confirmPassword"],
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
      // Mock API call
      // const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/register`, { ... })
      await new Promise(resolve => setTimeout(resolve, 1500))
      
      toast.success("Account created successfully!")
      router.push("/verify")
    } catch (error) {
      toast.error("Failed to create account. Please try again.")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen bg-bg-void text-text-primary">
      {/* Left side - 3D & Branding (Hidden on mobile) */}
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
            Join the private economy.
          </h1>
          <p className="text-text-secondary text-lg">
            Pay utilities with Zcash. No KYC. No middlemen. Just you and your tokens.
          </p>
        </div>
      </div>

      {/* Right side - Form */}
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
            <h2 className="font-dela text-3xl mb-2">Sign Up</h2>
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
                  leftIcon={<Mail className="h-5 w-5" />}
                  error={errors.email?.message}
                />
              </div>

              <div className="space-y-1">
                <Input
                  {...register("displayName")}
                  type="text"
                  placeholder="Display name (optional)"
                  leftIcon={<User className="h-5 w-5" />}
                  error={errors.displayName?.message}
                />
              </div>
              
              <div className="space-y-1">
                <Input
                  {...register("password")}
                  type="password"
                  placeholder="Password"
                  leftIcon={<Lock className="h-5 w-5" />}
                  error={errors.password?.message}
                />
              </div>

              <div className="space-y-1">
                <Input
                  {...register("confirmPassword")}
                  type="password"
                  placeholder="Confirm password"
                  leftIcon={<Lock className="h-5 w-5" />}
                  error={errors.confirmPassword?.message}
                />
              </div>

              <Button type="submit" className="w-full h-12 text-base mt-4" loading={isLoading}>
                Create Account <ArrowRight className="ml-2 h-4 w-4" />
              </Button>

              <p className="text-xs text-text-muted text-center mt-6">
                By signing up, you agree to our Terms of Service and Privacy Policy.
                We never store your wallet addresses.
              </p>
            </form>
          </motion.div>
        </div>
      </div>
    </div>
  )
}
