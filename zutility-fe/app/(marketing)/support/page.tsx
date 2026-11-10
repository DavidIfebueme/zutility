"use client"

import * as React from "react"
import Link from "next/link"
import { useForm } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { ArrowRight, Check, Clock, Mail } from "lucide-react"
import { SiteHeader } from "@/components/marketing/site-header"
import { SiteFooter } from "@/components/marketing/site-footer"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { apiPost } from "@/lib/api"

const supportSchema = z.object({
  email: z.string().email("Invalid email address"),
  name: z.string().min(1, "Name is required"),
  orderId: z.string().optional(),
  subject: z.string().min(1, "Subject is required"),
  message: z.string().min(10, "Message must be at least 10 characters"),
})

type SupportFormValues = z.infer<typeof supportSchema>

const BEFORE = [
  "Open your order page first. It shows the live state, the deposit address, and the confirmation count.",
  "Signed in, the same order sits in your history with its receipt.",
  "A transparent payment clears at 9 confirmations. A shielded payment clears at 10.",
  "Blocks arrive about every 25 seconds, so the wait is roughly four minutes.",
  "If the countdown has ended, send the order ID anyway. Support can still trace the payment.",
]

export default function SupportPage() {
  const [isLoading, setIsLoading] = React.useState(false)
  const [submitted, setSubmitted] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [sentOrderId, setSentOrderId] = React.useState<string | null>(null)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<SupportFormValues>({
    resolver: zodResolver(supportSchema),
  })

  const onSubmit = async (data: SupportFormValues) => {
    setIsLoading(true)
    setError(null)
    try {
      const orderId = data.orderId?.trim()
      await apiPost("/api/v1/support", {
        email: data.email,
        name: data.name,
        subject: data.subject,
        message: orderId ? `Order ${orderId}\n\n${data.message}` : data.message,
      })
      setSentOrderId(orderId || null)
      setSubmitted(true)
    } catch {
      setError("The message did not go through. Try again, or send it once more in a moment.")
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-paper">
      <SiteHeader />

      <main>
        <section className="paper-grid border-b border-line">
          <div className="mx-auto max-w-6xl px-5 py-14 lg:py-16">
            <p className="label-xs text-gold-deep">Support</p>
            <h1 className="mt-4 max-w-2xl text-4xl leading-[1.1] text-ink sm:text-5xl">
              Tell us the order, and what went wrong.
            </h1>
            <p className="mt-4 max-w-2xl text-[15px] leading-relaxed text-ink-2">
              Most order problems clear once we can see the payment against its deposit address.
              The order ID is the fastest way to get there.
            </p>
          </div>
        </section>

        <section className="border-b border-line">
          <div className="mx-auto grid max-w-6xl gap-12 px-5 py-14 lg:grid-cols-[340px_1fr] lg:gap-16 lg:py-20">
            <div>
              <p className="label-xs text-muted">Before you write</p>
              <ul className="mt-4 space-y-3 border-t border-line pt-4">
                {BEFORE.map((line) => (
                  <li key={line} className="flex items-start gap-2.5 text-[13px] leading-relaxed text-ink-2">
                    <Check className="mt-0.5 h-4 w-4 shrink-0 text-green" strokeWidth={1.8} />
                    {line}
                  </li>
                ))}
              </ul>

              <div className="mt-8 flex items-start gap-3 rounded-lg border border-line bg-cream px-4 py-3">
                <Clock className="mt-0.5 h-4 w-4 shrink-0 text-gold-deep" strokeWidth={1.6} />
                <p className="text-[13px] leading-relaxed text-ink-2">
                  A payment that has not reached the confirmation count yet is not late. Check the
                  order page before you send a second payment.
                </p>
              </div>

              <p className="mt-6 text-[13px] leading-relaxed text-muted">
                Signed in, your order history holds every order you have started.
              </p>
              <Link
                href="/history"
                className="mt-2 inline-flex items-center gap-1.5 text-[13px] text-gold-deep underline underline-offset-4 hover:no-underline"
              >
                Open order history
                <ArrowRight className="h-4 w-4" strokeWidth={1.8} />
              </Link>
            </div>

            <div className="rounded-lg border border-line bg-cream p-6 sm:p-8">
              {submitted ? (
                <div aria-live="polite">
                  <span className="flex h-10 w-10 items-center justify-center rounded-md bg-gold/20 text-gold-deep">
                    <Check className="h-5 w-5" strokeWidth={1.8} />
                  </span>
                  <h2 className="mt-4 text-2xl text-ink">Message sent.</h2>
                  <p className="mt-2 max-w-md text-[14px] leading-relaxed text-ink-2">
                    We reply by email to the address you gave. Keep the thread together if you write
                    again.
                  </p>
                  {sentOrderId && (
                    <div className="mt-5 rounded-md border border-line bg-paper px-4 py-3">
                      <p className="label-xs text-muted">Order on the message</p>
                      <p className="num mt-1 text-[14px] text-ink">{sentOrderId}</p>
                    </div>
                  )}
                  <div className="mt-6 flex flex-wrap gap-3">
                    <Button asChild variant="secondary">
                      <Link href="/">Back to the home page</Link>
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={() => {
                        setSubmitted(false)
                        setSentOrderId(null)
                      }}
                    >
                      Write another message
                    </Button>
                  </div>
                </div>
              ) : (
                <>
                  <div className="flex items-start gap-3 border-b border-line pb-5">
                    <span className="flex h-9 w-9 items-center justify-center rounded-md bg-paper-2 text-ink-2">
                      <Mail className="h-4 w-4" strokeWidth={1.6} />
                    </span>
                    <div>
                      <h2 className="text-xl text-ink">Send a message</h2>
                      <p className="mt-1 text-[13px] text-muted">
                        Every field except the order ID is required.
                      </p>
                    </div>
                  </div>

                  <form onSubmit={handleSubmit(onSubmit)} className="mt-6 space-y-4" noValidate>
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div className="space-y-1.5">
                        <label htmlFor="name" className="block text-[13px] font-medium text-ink">
                          Name
                        </label>
                        <Input
                          id="name"
                          {...register("name")}
                          type="text"
                          autoComplete="name"
                          placeholder="Your name"
                          error={errors.name?.message}
                        />
                      </div>

                      <div className="space-y-1.5">
                        <label htmlFor="email" className="block text-[13px] font-medium text-ink">
                          Email
                        </label>
                        <Input
                          id="email"
                          {...register("email")}
                          type="email"
                          autoComplete="email"
                          placeholder="you@example.com"
                          error={errors.email?.message}
                        />
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <label htmlFor="orderId" className="block text-[13px] font-medium text-ink">
                        Order ID <span className="font-normal text-muted">(optional)</span>
                      </label>
                      <Input
                        id="orderId"
                        {...register("orderId")}
                        type="text"
                        placeholder="8F2C-41A9"
                        className="num"
                        error={errors.orderId?.message}
                      />
                    </div>

                    <div className="space-y-1.5">
                      <label htmlFor="subject" className="block text-[13px] font-medium text-ink">
                        Subject
                      </label>
                      <Input
                        id="subject"
                        {...register("subject")}
                        type="text"
                        placeholder="Failed delivery, wrong meter, underpayment"
                        error={errors.subject?.message}
                      />
                    </div>

                    <div className="space-y-1.5">
                      <label htmlFor="message" className="block text-[13px] font-medium text-ink">
                        Message
                      </label>
                      <textarea
                        id="message"
                        {...register("message")}
                        rows={6}
                        placeholder="What you paid, when you sent it, and what the order page shows."
                        className="w-full resize-none rounded-lg border border-line bg-cream px-3 py-2.5 text-sm text-ink placeholder:text-muted/70 transition-colors focus-visible:border-line-strong focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-gold-deep"
                      />
                      {errors.message && (
                        <p className="text-xs text-rust">{errors.message.message}</p>
                      )}
                    </div>

                    {error && (
                      <p className="text-[13px] text-rust" aria-live="polite">
                        {error}
                      </p>
                    )}

                    <div className="flex flex-wrap items-center gap-4 pt-1">
                      <Button type="submit" size="lg" loading={isLoading}>
                        Send message
                        <ArrowRight className="h-4 w-4" strokeWidth={1.8} />
                      </Button>
                      <p className="text-[12px] text-muted">
                        We use the email only to answer this message.
                      </p>
                    </div>
                  </form>
                </>
              )}
            </div>
          </div>
        </section>
      </main>

      <SiteFooter />
    </div>
  )
}
