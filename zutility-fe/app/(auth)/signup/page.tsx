"use client"

import Link from "next/link"
import { motion } from "motion/react"
import LockIcon from "@/components/icons/lock-icon"
import { Button } from "@/components/ui/button"
import { PaymentFlow } from "@/components/ui/payment-flow"

export default function SignupPage() {
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
            className="text-center"
          >
            <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
              <LockIcon size={32} />
            </div>
            <h2 className="font-dela text-3xl mb-2">Registration Closed</h2>
            <p className="text-text-secondary mb-8">
              We&apos;re not accepting new accounts right now. Join the waitlist to get early access.
            </p>
            <div className="space-y-3">
              <Link href="/waitlist">
                <Button variant="primary" className="w-full">
                  Join the Waitlist
                </Button>
              </Link>
              <Link href="/login">
                <Button variant="secondary" className="w-full">
                  Already have an account? Log in
                </Button>
              </Link>
            </div>
          </motion.div>
        </div>
      </div>
    </div>
  )
}
