import Link from "next/link"
import { Lock } from "lucide-react"
import { WordmarkLink } from "@/components/ui/wordmark"
import { ThemeToggle } from "@/components/theme-provider"
import { cn } from "@/lib/utils"

interface AuthShellProps {
  title: string
  subtitle: string
  children: React.ReactNode
  footer?: React.ReactNode
  shake?: boolean
}

export function AuthShell({ title, subtitle, children, footer, shake }: AuthShellProps) {
  return (
    <div className="paper-grid flex min-h-screen flex-col bg-paper">
      <div className="mx-auto flex w-full max-w-6xl items-center justify-between px-5 py-5">
        <WordmarkLink />
        <div className="flex items-center gap-2">
          <Link href="/pay" className="text-[13px] text-muted transition-colors hover:text-ink">
            Pay without an account
          </Link>
          <ThemeToggle />
        </div>
      </div>

      <div className="flex flex-1 items-start justify-center px-5 pb-16 pt-6 sm:items-center sm:pt-0">
        <div className="w-full max-w-[420px]">
          <div
            className={cn(
              "form-card reveal-in rounded-lg border border-line bg-cream p-6 sm:p-7 transition-colors duration-200 hover:border-line-strong",
              shake && "animate-shake"
            )}
          >
            <h1 className="text-2xl text-ink">{title}</h1>
            <p className="mt-1.5 text-[13px] leading-relaxed text-muted">{subtitle}</p>
            <div className="mt-6">{children}</div>
          </div>

          {footer}

          <p className="mt-5 flex items-center justify-center gap-1.5 text-[12px] text-muted">
            <Lock className="h-3.5 w-3.5" strokeWidth={1.6} />
            No identity documents. No KYC.
          </p>
        </div>
      </div>
    </div>
  )
}

export function AuthFooterLink({ prompt, href, label }: { prompt: string; href: string; label: string }) {
  return (
    <p className="mt-4 text-center text-[13px] text-muted">
      {prompt}{" "}
      <Link href={href} className="text-gold-deep u-link">
        {label}
      </Link>
    </p>
  )
}
