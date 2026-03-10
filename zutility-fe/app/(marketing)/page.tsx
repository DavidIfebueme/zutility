"use client"

import * as React from "react"
import Link from "next/link"
import { motion, useScroll, useTransform } from "motion/react"
import { ArrowRight, Shield, Zap, Lock, ChevronDown, Menu, X, ArrowRightLeft, Store } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { TokenBadge } from "@/components/ui/token-badge"
import { RateTicker } from "@/components/ui/rate-ticker"
import { UTILITIES, TOKENS } from "@/lib/constants"
import dynamic from "next/dynamic"

const ZecCoinScene = dynamic(() => import("@/components/3d/ZecCoin").then(mod => mod.ZecCoinScene), { ssr: false })
const ParticleField = dynamic(() => import("@/components/3d/ParticleField").then(mod => mod.ParticleField), { ssr: false })
const WireframeSphere = dynamic(() => import("@/components/3d/WireframeSphere").then(mod => mod.WireframeSphere), { ssr: false })
const Canvas = dynamic(() => import("@react-three/fiber").then(mod => mod.Canvas), { ssr: false })

export default function LandingPage() {
  const { scrollY } = useScroll()
  const navBackground = useTransform(scrollY, [0, 50], ["rgba(5, 5, 8, 0)", "rgba(5, 5, 8, 0.8)"])
  const navBorder = useTransform(scrollY, [0, 50], ["rgba(30, 30, 46, 0)", "rgba(30, 30, 46, 1)"])
  const navBlur = useTransform(scrollY, [0, 50], ["blur(0px)", "blur(12px)"])

  const [isMobileMenuOpen, setIsMobileMenuOpen] = React.useState(false)
  const [openFaq, setOpenFaq] = React.useState<number | null>(null)

  const faqs = [
    { q: "Do I need an account?", a: "Yes, a simple email signup is required to track your orders and history. However, no KYC or identity verification is needed for utility payments." },
    { q: "What wallets are supported?", a: "Any wallet that supports Zcash (ZEC). We recommend Zashi or Nighthawk Wallet for the best shielded transaction experience." },
    { q: "Shielded vs Transparent?", a: "Shielded (z-addresses) offer full privacy but take ~13 minutes to confirm. Transparent (t-addresses) are standard public transactions and confirm in ~4 minutes." },
    { q: "How long does payment take?", a: "Once we detect your transaction on the blockchain (usually within seconds), we wait for the required confirmations. Then your utility is delivered instantly." },
    { q: "What if I send the wrong amount?", a: "Our system expects the exact ZEC amount shown. If you send a different amount, the order will be flagged for manual review and may take up to 24 hours to resolve." },
    { q: "Is this available everywhere in Nigeria?", a: "Yes, as long as you are paying for a supported Nigerian utility (MTN, DSTV, PHCN, etc.), you can use zutility from anywhere." },
    { q: "Are other tokens coming?", a: "Yes. We are starting with Zcash (ZEC) to establish the privacy-first foundation, but will be adding other ZK and privacy-preserving tokens soon." },
  ]

  return (
    <div className="min-h-screen bg-bg-void text-text-primary selection:bg-accent-zec/30">
      {/* Navigation */}
      <motion.nav
        style={{ backgroundColor: navBackground, borderBottomColor: navBorder, backdropFilter: navBlur }}
        className="fixed top-0 left-0 right-0 z-50 border-b border-transparent transition-colors"
      >
        <div className="mx-auto flex h-20 max-w-7xl items-center justify-between px-6">
          <Link href="/" className="font-dela text-2xl tracking-tight">
            <span className="text-accent-zec">z</span>utility
          </Link>

          <div className="hidden items-center gap-8 md:flex">
            <Link href="/how-it-works" className="text-sm font-medium text-text-secondary transition-colors hover:text-text-primary">
              How it works
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

          <button
            className="md:hidden text-text-secondary hover:text-text-primary"
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          >
            {isMobileMenuOpen ? <X /> : <Menu />}
          </button>
        </div>
      </motion.nav>

      {/* Mobile Menu Overlay */}
      {isMobileMenuOpen && (
        <div className="fixed inset-0 z-40 bg-bg-void/95 pt-24 px-6 backdrop-blur-md md:hidden">
          <div className="flex flex-col gap-6">
            <Link href="/how-it-works" className="text-lg font-medium text-text-secondary" onClick={() => setIsMobileMenuOpen(false)}>
              How it works
            </Link>
            <div className="h-px bg-border-subtle" />
            <Link href="/login" onClick={() => setIsMobileMenuOpen(false)}>
              <Button variant="ghost" className="w-full justify-start text-lg">Login</Button>
            </Link>
            <Link href="/signup" onClick={() => setIsMobileMenuOpen(false)}>
              <Button variant="primary" className="w-full text-lg">Get Started</Button>
            </Link>
          </div>
        </div>
      )}

      {/* Hero Section */}
      <section className="relative flex min-h-screen items-center pt-20 overflow-hidden">
        {/* 3D Background */}
        <div className="absolute inset-0 z-0 opacity-50">
          <Canvas camera={{ position: [0, 0, 15], fov: 60 }}>
            <ParticleField />
          </Canvas>
        </div>

        <div className="mx-auto grid w-full max-w-7xl grid-cols-1 gap-12 px-6 lg:grid-cols-2 lg:gap-8 relative z-10">
          <div className="flex flex-col justify-center pt-12 lg:pt-0">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.6, ease: "easeOut" }}
            >
              <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-accent-zec/20 bg-accent-zec/5 px-3 py-1 text-xs font-medium tracking-widest text-accent-zec uppercase">
                <span className="relative flex h-2 w-2">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-accent-zec opacity-75"></span>
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-accent-zec"></span>
                </span>
                ZK Tokens → Naira & Utilities
              </div>
              
              <h1 className="font-dela text-5xl leading-[1.1] tracking-tight sm:text-6xl lg:text-7xl">
                Privacy Crypto.<br />
                Real Life.<br />
                <span className="text-text-muted">Nigeria.</span>
              </h1>
              
              <p className="mt-6 max-w-lg text-lg text-text-secondary sm:text-xl">
                Pay airtime, DSTV, electricity with Zcash. No KYC. No middlemen. The bridge between shielded tokens and everyday utility.
              </p>
              
              <div className="mt-10 flex flex-col gap-4 sm:flex-row sm:items-center">
                <Link href="/signup">
                  <Button size="lg" className="w-full sm:w-auto text-base h-14 px-8">
                    Pay a Utility
                  </Button>
                </Link>
                <Link href="/how-it-works">
                  <Button variant="secondary" size="lg" className="w-full sm:w-auto text-base h-14 px-8">
                    See How It Works
                  </Button>
                </Link>
              </div>

              <div className="mt-12 flex flex-wrap items-center gap-x-8 gap-y-4 text-sm font-medium text-text-muted">
                <div className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-accent-zec" />
                  No KYC Required
                </div>
                <div className="flex items-center gap-2">
                  <Zap className="h-4 w-4 text-accent-zec" />
                  ~4min Settlement
                </div>
                <div className="flex items-center gap-2">
                  <Lock className="h-4 w-4 text-accent-zec" />
                  Shielded ZEC Supported
                </div>
              </div>
            </motion.div>
          </div>

          <div className="relative hidden h-[600px] w-full items-center justify-center lg:flex">
            <div className="absolute inset-0 bg-gradient-to-r from-bg-void via-transparent to-transparent z-10 pointer-events-none" />
            <Canvas camera={{ position: [0, 0, 5], fov: 45 }} className="z-0">
              <ZecCoinScene />
            </Canvas>
          </div>
        </div>
      </section>

      {/* Products Section */}
      <section className="border-t border-border-subtle bg-bg-surface py-24 relative z-10">
        <div className="mx-auto max-w-7xl px-6">
          <div className="grid gap-8 md:grid-cols-3">
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.1 }}
            >
              <Card glow className="h-full bg-bg-elevated border-border-subtle hover:border-accent-zec/50 transition-colors">
                <CardHeader>
                  <div className="mb-4 flex items-center justify-between">
                    <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-accent-zec/10 text-accent-zec">
                      <Zap className="h-6 w-6" />
                    </div>
                    <Badge variant="live">LIVE</Badge>
                  </div>
                  <CardTitle className="text-xl">Utility Payments</CardTitle>
                  <CardDescription className="text-base mt-2">
                    Pay for MTN, Airtel, DSTV, and Electricity directly with ZEC. Instant delivery upon confirmation.
                  </CardDescription>
                </CardHeader>
              </Card>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.2 }}
            >
              <Card className="h-full bg-bg-elevated border-border-subtle opacity-80">
                <CardHeader>
                  <div className="mb-4 flex items-center justify-between">
                    <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-text-muted/20 text-text-secondary">
                      <ArrowRightLeft className="h-6 w-6" />
                    </div>
                    <Badge variant="coming-soon">COMING SOON</Badge>
                  </div>
                  <CardTitle className="text-xl">OTC Off-ramp</CardTitle>
                  <CardDescription className="text-base mt-2">
                    Directly swap ZEC for Naira at our system rate. Fast settlement directly to your Nigerian bank account.
                  </CardDescription>
                </CardHeader>
              </Card>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.3 }}
            >
              <Card className="h-full bg-bg-elevated border-border-subtle opacity-80">
                <CardHeader>
                  <div className="mb-4 flex items-center justify-between">
                    <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-text-muted/20 text-text-secondary">
                      <Store className="h-6 w-6" />
                    </div>
                    <Badge variant="coming-soon">COMING SOON</Badge>
                  </div>
                  <CardTitle className="text-xl">P2P Marketplace</CardTitle>
                  <CardDescription className="text-base mt-2">
                    Peer-to-peer ZEC/NGN escrow trading. Set your own rates and trade securely with other users.
                  </CardDescription>
                </CardHeader>
              </Card>
            </motion.div>
          </div>
        </div>
      </section>

      {/* ZK Tokens Section */}
      <section className="py-24 relative z-10">
        <div className="mx-auto max-w-4xl px-6 text-center">
          <h2 className="font-dela text-3xl sm:text-4xl mb-12">Starting with Zcash. Built for the ZK ecosystem.</h2>
          
          <div className="flex flex-wrap items-center justify-center gap-4 mb-12">
            {TOKENS.map((token) => (
              <TokenBadge key={token.id} token={token.status === 'live' ? 'ZEC' : 'COMING_SOON'} />
            ))}
          </div>

          <p className="text-lg text-text-secondary max-w-2xl mx-auto">
            We believe financial privacy is a fundamental right. By bridging shielded ZK tokens to real-world utility, we enable Nigerians to transact freely without surveillance.
          </p>
        </div>
      </section>

      {/* How It Works Section */}
      <section className="relative py-32 overflow-hidden bg-bg-surface border-y border-border-subtle z-10">
        <div className="absolute inset-0 z-0 opacity-30 pointer-events-none hidden md:block">
          <Canvas camera={{ position: [0, 0, 5], fov: 45 }}>
            <ambientLight intensity={0.5} />
            <WireframeSphere />
          </Canvas>
        </div>

        <div className="mx-auto max-w-7xl px-6 relative z-10">
          <div className="text-center mb-20">
            <h2 className="font-dela text-4xl sm:text-5xl mb-6">How It Works</h2>
            <p className="text-xl text-text-secondary">Three simple steps to bridge your privacy tokens to reality.</p>
          </div>

          <div className="grid gap-12 md:grid-cols-3">
            {[
              { step: "01", title: "Choose Utility", desc: "Select what you want to pay for (Airtime, DSTV, Electricity) and enter your details." },
              { step: "02", title: "Send ZEC", desc: "Send the exact ZEC amount to the unique deposit address provided. Shielded or transparent." },
              { step: "03", title: "Service Delivered", desc: "Our system detects the payment on-chain and instantly delivers your utility or token." }
            ].map((item, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: i * 0.2 }}
                className="relative"
              >
                <div className="text-6xl font-dela text-bg-elevated mb-6 select-none">{item.step}</div>
                <div className="absolute top-6 left-4">
                  <h3 className="text-2xl font-semibold mb-3 font-dela text-accent-zec">{item.title}</h3>
                  <p className="text-text-secondary leading-relaxed">{item.desc}</p>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Utilities Grid */}
      <section className="py-32 relative z-10">
        <div className="mx-auto max-w-7xl px-6">
          <div className="flex flex-col md:flex-row md:items-end justify-between mb-16 gap-6">
            <div>
              <h2 className="font-dela text-4xl mb-4">Supported Utilities</h2>
              <p className="text-text-secondary text-lg">Pay for everyday services directly with Zcash.</p>
            </div>
            <Link href="/signup">
              <Button variant="secondary" className="border-accent-zec text-accent-zec hover:bg-accent-zec hover:text-bg-void">
                View All Services <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </Link>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {UTILITIES.map((u, i) => (
              <motion.div
                key={u.id}
                initial={{ opacity: 0, scale: 0.95 }}
                whileInView={{ opacity: 1, scale: 1 }}
                viewport={{ once: true }}
                transition={{ duration: 0.4, delay: i * 0.05 }}
              >
                <Card className="h-full hover:-translate-y-1 transition-transform duration-300 cursor-pointer border-border-subtle hover:border-accent-zec/50 bg-bg-elevated">
                  <CardContent className="p-6 flex flex-col items-center text-center gap-4">
                    <div className="h-12 w-12 rounded-full bg-bg-surface flex items-center justify-center text-xl font-bold text-text-muted">
                      {u.name.charAt(0)}
                    </div>
                    <div>
                      <h4 className="font-semibold text-text-primary">{u.name}</h4>
                      <p className="text-xs text-text-secondary mt-1">{u.descriptor}</p>
                    </div>
                  </CardContent>
                </Card>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Live Rate Section */}
      <section className="bg-bg-elevated py-24 border-y border-border-subtle relative z-10">
        <div className="mx-auto max-w-3xl px-6 text-center">
          <h2 className="font-dela text-3xl mb-8">Live Exchange Rate</h2>
          <div className="inline-flex items-center justify-center p-6 rounded-2xl bg-bg-void border border-border-subtle shadow-xl mb-6">
            <RateTicker className="text-2xl sm:text-4xl" />
          </div>
          <p className="text-text-muted text-sm">
            Rates are updated every 60 seconds. When you create an order, the rate is locked for 15 minutes.
          </p>
        </div>
      </section>

      {/* FAQ Section */}
      <section className="py-32 relative z-10">
        <div className="mx-auto max-w-3xl px-6">
          <h2 className="font-dela text-4xl mb-16 text-center">Frequently Asked Questions</h2>
          
          <div className="space-y-4">
            {faqs.map((faq, i) => (
              <div key={i} className="border border-border-subtle rounded-lg bg-bg-surface overflow-hidden">
                <button
                  className="w-full px-6 py-4 flex items-center justify-between text-left font-medium focus:outline-none"
                  onClick={() => setOpenFaq(openFaq === i ? null : i)}
                >
                  <span className="text-lg">{faq.q}</span>
                  <ChevronDown className={cn("h-5 w-5 text-text-muted transition-transform duration-200", openFaq === i && "rotate-180")} />
                </button>
                <motion.div
                  initial={false}
                  animate={{ height: openFaq === i ? "auto" : 0, opacity: openFaq === i ? 1 : 0 }}
                  className="overflow-hidden"
                >
                  <div className="px-6 pb-4 text-text-secondary leading-relaxed">
                    {faq.a}
                  </div>
                </motion.div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border-subtle bg-bg-surface py-12 relative z-10">
        <div className="mx-auto max-w-7xl px-6 flex flex-col md:flex-row justify-between items-center gap-6">
          <div className="flex flex-col items-center md:items-start gap-2">
            <Link href="/" className="font-dela text-2xl tracking-tight">
              <span className="text-accent-zec">z</span>utility
            </Link>
            <p className="text-sm text-text-muted">ZK tokens. Nigerian utilities. No compromise.</p>
          </div>
          
          <div className="flex gap-6 text-sm font-medium text-text-secondary">
            <Link href="/how-it-works" className="hover:text-text-primary transition-colors">How it works</Link>
            <Link href="/login" className="hover:text-text-primary transition-colors">Login</Link>
            <a href="#" className="hover:text-text-primary transition-colors">Support</a>
            <a href="#" className="hover:text-text-primary transition-colors">Terms</a>
          </div>
        </div>
        <div className="mx-auto max-w-7xl px-6 mt-12 pt-8 border-t border-border-subtle flex flex-col md:flex-row justify-between items-center gap-4 text-xs text-text-muted">
          <p>© 2025 zutility. All rights reserved.</p>
          <p>Disclaimer: zutility is a non-custodial platform and not a registered bank.</p>
        </div>
      </footer>
    </div>
  )
}
