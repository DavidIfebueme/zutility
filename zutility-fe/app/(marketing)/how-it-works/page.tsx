"use client"

import * as React from "react"
import Link from "next/link"
import { motion } from "motion/react"
import ArrowNarrowRightIcon from "@/components/icons/arrow-narrow-right-icon"
import ShieldCheck from "@/components/icons/shield-check"
import PlugConnectedIcon from "@/components/icons/plug-connected-icon"
import LockIcon from "@/components/icons/lock-icon"
import ClockIcon from "@/components/icons/clock-icon"
import QrcodeIcon from "@/components/icons/qrcode-icon"
import CheckedIcon from "@/components/icons/checked-icon"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

const steps = [
  {
    step: "01",
    title: "Choose Your Utility",
    desc: "Select what you want to pay for — airtime, data, DSTV, GOtv, Startimes, electricity, WAEC, JAMB, or school fees. Enter your meter number, smartcard number, or phone number.",
    icon: ShieldCheck,
  },
  {
    step: "02",
    title: "Validate & Confirm",
    desc: "We verify your account details with the provider in real time. You'll see the customer name and amount before paying. No surprises.",
    icon: CheckedIcon,
  },
  {
    step: "03",
    title: "Send ZEC",
    desc: "A unique Zcash deposit address and exact ZEC amount are generated for your order. Send from any Zcash wallet — shielded or transparent. Scan the QR code or copy the address.",
    icon: QrcodeIcon,
  },
  {
    step: "04",
    title: "We Detect & Deliver",
    desc: "Our system watches the blockchain and detects your payment within seconds. After 3 confirmations (~4 minutes), your utility is delivered instantly. You'll see real-time status updates.",
    icon: PlugConnectedIcon,
  },
]

export default function HowItWorksPage() {
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
            <Link href="/signup">
              <Button variant="primary">Get Started</Button>
            </Link>
          </div>
        </div>
      </div>

      <section className="relative flex items-center py-24 overflow-hidden">
        <div className="absolute right-0 top-0 w-1/2 h-full opacity-20 pointer-events-none hidden lg:block">
          <Canvas camera={{ position: [0, 0, 5], fov: 45 }}>
            <ambientLight intensity={0.5} />
            <ZecCoinScene />
          </Canvas>
        </div>

        <div className="mx-auto max-w-4xl px-6 relative z-10">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
            className="text-center mb-20"
          >
            <h1 className="font-dela text-5xl sm:text-6xl mb-6">How It Works</h1>
            <p className="text-xl text-text-secondary max-w-2xl mx-auto">
              From Zcash to utility delivery in under 4 minutes. No account verification. No KYC. Just send and receive.
            </p>
          </motion.div>

          <div className="space-y-8">
            {steps.map((item, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, x: -20 }}
                whileInView={{ opacity: 1, x: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.15 }}
              >
                <Card className="border-border-subtle bg-bg-elevated hover:border-accent-zec/30 transition-colors">
                  <CardContent className="p-8 flex flex-col md:flex-row items-start gap-6">
                    <div className="flex items-center gap-4 shrink-0">
                      <div className="text-4xl font-dela text-accent-zec">{item.step}</div>
                      <div className="h-12 w-12 rounded-lg bg-accent-zec/10 flex items-center justify-center text-accent-zec">
                        <item.icon size={24} />
                      </div>
                    </div>
                    <div>
                      <h3 className="text-xl font-semibold mb-2 font-dela">{item.title}</h3>
                      <p className="text-text-secondary leading-relaxed">{item.desc}</p>
                    </div>
                  </CardContent>
                </Card>
              </motion.div>
            ))}
          </div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="mt-16"
          >
            <Card className="border-accent-zec/20 bg-bg-surface">
              <CardHeader className="text-center">
                <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-accent-zec/10 text-accent-zec">
                  <ClockIcon size={28} />
                </div>
                <CardTitle className="text-2xl font-dela">Timing Breakdown</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 sm:grid-cols-3 text-center">
                  <div className="rounded-lg bg-bg-elevated p-4 border border-border-subtle">
                    <p className="text-2xl font-dela text-accent-zec">&lt;1s</p>
                    <p className="text-sm text-text-secondary mt-1">Payment detection</p>
                  </div>
                  <div className="rounded-lg bg-bg-elevated p-4 border border-border-subtle">
                    <p className="text-2xl font-dela text-accent-zec">~4min</p>
                    <p className="text-sm text-text-secondary mt-1">3 confirmations</p>
                  </div>
                  <div className="rounded-lg bg-bg-elevated p-4 border border-border-subtle">
                    <p className="text-2xl font-dela text-accent-zec">Instant</p>
                    <p className="text-sm text-text-secondary mt-1">Utility delivery</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            className="mt-16"
          >
            <Card className="border-border-subtle bg-bg-elevated">
              <CardHeader className="text-center">
                <CardTitle className="text-2xl font-dela">Privacy Features</CardTitle>
                <CardDescription className="text-base mt-2">Built for a world where financial privacy matters.</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="flex items-start gap-3 p-4 rounded-lg bg-bg-void border border-border-subtle">
                    <LockIcon size={20} className="text-accent-zec shrink-0 mt-0.5" />
                    <div>
                      <p className="font-medium">Shielded ZEC</p>
                      <p className="text-sm text-text-secondary mt-1">Send from z-addresses for full transaction privacy. Your sender address and amount are encrypted on-chain.</p>
                    </div>
                  </div>
                  <div className="flex items-start gap-3 p-4 rounded-lg bg-bg-void border border-border-subtle">
                    <ShieldCheck size={20} className="text-accent-zec shrink-0 mt-0.5" />
                    <div>
                      <p className="font-medium">No KYC Required</p>
                      <p className="text-sm text-text-secondary mt-1">We don't ask for your ID, passport, or proof of address. Just an email to track your orders.</p>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </motion.div>

          <div className="mt-16 text-center">
            <Link href="/signup">
              <Button size="lg" className="text-base h-14 px-8">
                Get Started <ArrowNarrowRightIcon size={16} className="ml-2" />
              </Button>
            </Link>
          </div>
        </div>
      </section>

      <footer className="border-t border-border-subtle bg-bg-surface py-8 mt-16">
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
