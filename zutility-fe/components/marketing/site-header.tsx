import Link from "next/link"
import { Button } from "@/components/ui/button"
import { WordmarkLink } from "@/components/ui/wordmark"
import { RateTicker } from "@/components/ui/rate-ticker"
import { ThemeToggle } from "@/components/theme-provider"

const NAV = [
  { href: "/how-it-works", label: "How it works" },
  { href: "/#rates", label: "Rates" },
  { href: "/support", label: "Support" },
]

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-40 border-b border-line bg-paper/90 backdrop-blur-sm">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-6 px-5">
        <div className="flex items-center gap-8">
          <WordmarkLink />
          <nav className="hidden items-center gap-6 md:flex" aria-label="Main">
            {NAV.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className="u-link text-[13px] text-muted transition-colors hover:text-ink"
              >
                {item.label}
              </Link>
            ))}
          </nav>
        </div>
        <div className="flex items-center gap-1.5">
          <RateTicker className="mr-2 hidden lg:flex" />
          <ThemeToggle />
          <Button asChild variant="ghost" size="sm">
            <Link href="/login">Sign in</Link>
          </Button>
          <Button asChild size="sm">
            <Link href="/pay">Pay a bill</Link>
          </Button>
        </div>
      </div>
    </header>
  )
}
