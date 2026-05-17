"use client"

import * as React from "react"
import Link from "next/link"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { motion } from "motion/react"
import { Mail, MessageSquare, User, Send, CheckCircle2, ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { apiPost } from "@/lib/api"
import { toast } from "sonner"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const supportSchema = z.object({
  email: z.string().email("Invalid email address"),
  name: z.string().min(1, "Name is required"),
  subject: z.string().min(1, "Subject is required"),
  message: z.string().min(10, "Message must be at least 10 characters"),
})

type SupportFormValues = z.infer<typeof supportSchema>

export default function SupportPage() {
  const [isLoading, setIsLoading] = React.useState(false)
  const [submitted, setSubmitted] = React.useState(false)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<SupportFormValues>({
    resolver: zodResolver(supportSchema),
  })

  const onSubmit = async (data: SupportFormValues) => {
    setIsLoading(true)
    try {
      await apiPost("/api/v1/support", {
        email: data.email,
        name: data.name,
        subject: data.subject,
        message: data.message,
      })
      setSubmitted(true)
      toast.success("Message sent!")
    } catch {
      toast.error("Failed to send message. Please try again.")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-bg-void text-text-primary">
      <div className="border-b border-border-subtle bg-bg-surface/50 backdrop-blur-md sticky top-0 z-50">
        <div className="mx-auto flex h-20 max-w-7xl items-center justify-between px-6">
          <Link href="/" className="font-dela text-2xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
          <div className="flex items-center gap-4">
            <Link href="/login">
              <Button variant="ghost">Login</Button>
            </Link>
          </div>
        </div>
      </div>

      <section className="relative flex items-center py-24 overflow-hidden">
        <div className="absolute right-0 top-0 w-1/3 h-full opacity-15 pointer-events-none hidden lg:block">
          <Canvas camera={{ position: [0, 0, 5], fov: 45 }}>
            <ambientLight intensity={0.5} />
            <ZecCoinScene />
          </Canvas>
        </div>

        <div className="mx-auto max-w-xl px-6 relative z-10 w-full">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
          >
            {submitted ? (
              <div className="text-center space-y-6">
                <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
                  <CheckCircle2 className="h-8 w-8" />
                </div>
                <h1 className="font-dela text-3xl">Message Sent</h1>
                <p className="text-text-secondary">
                  We&apos;ll get back to you within 24 hours. Check your email for a confirmation.
                </p>
                <Link href="/">
                  <Button variant="secondary">
                    Back to Home <ArrowRight className="ml-2 h-4 w-4" />
                  </Button>
                </Link>
              </div>
            ) : (
              <>
                <div className="text-center mb-10">
                  <h1 className="font-dela text-4xl mb-3">Contact Support</h1>
                  <p className="text-text-secondary">
                    Have an issue with an order or a question? We&apos;re here to help.
                  </p>
                </div>

                <Card className="border-border-subtle bg-bg-elevated">
                  <CardContent className="p-8">
                    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
                      <div className="space-y-1">
                        <Input
                          {...register("name")}
                          type="text"
                          placeholder="Your name"
                          leftIcon={<User className="h-5 w-5" />}
                          error={errors.name?.message}
                        />
                      </div>

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
                          {...register("subject")}
                          type="text"
                          placeholder="Subject (e.g. Order #abc - failed delivery)"
                          leftIcon={<MessageSquare className="h-5 w-5" />}
                          error={errors.subject?.message}
                        />
                      </div>

                      <div className="space-y-1">
                        <textarea
                          {...register("message")}
                          rows={5}
                          placeholder="Describe your issue... Include your order ID if applicable."
                          className="w-full rounded-lg border border-border-subtle bg-bg-void px-4 py-3 text-sm text-text-primary placeholder:text-text-muted focus:border-accent-zec focus:outline-none focus:ring-1 focus:ring-accent-zec/50 resize-none"
                        />
                        {errors.message && (
                          <p className="text-xs text-red-400 mt-1">{errors.message.message}</p>
                        )}
                      </div>

                      <Button type="submit" className="w-full h-12 text-base" loading={isLoading}>
                        Send Message <Send className="ml-2 h-4 w-4" />
                      </Button>
                    </form>
                  </CardContent>
                </Card>
              </>
            )}
          </motion.div>
        </div>
      </section>

      <footer className="border-t border-border-subtle bg-bg-surface py-8">
        <div className="mx-auto max-w-7xl px-6 flex flex-col md:flex-row justify-between items-center gap-4 text-sm text-text-secondary">
          <Link href="/" className="font-dela text-xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>
          <div className="flex gap-6">
            <Link href="/how-it-works" className="hover:text-text-primary transition-colors">How it works</Link>
            <Link href="/support" className="hover:text-text-primary transition-colors">Support</Link>
          </div>
        </div>
      </footer>
    </div>
  )
}
