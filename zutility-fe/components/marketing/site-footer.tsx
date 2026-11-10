import Link from "next/link"
import { WordmarkLink } from "@/components/ui/wordmark"

const COLUMNS = [
  {
    title: "Product",
    links: [
      { href: "/pay", label: "Pay a bill" },
      { href: "/how-it-works", label: "How it works" },
      { href: "/dashboard", label: "Dashboard" },
    ],
  },
  {
    title: "Company",
    links: [
      { href: "/support", label: "Support" },
      { href: "/waitlist", label: "Waitlist" },
      { href: "/#rates", label: "Rates" },
    ],
  },
]

export function SiteFooter() {
  return (
    <footer className="border-t border-line bg-paper-2">
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-12 sm:grid-cols-2 lg:grid-cols-4">
        <div className="sm:col-span-2">
          <WordmarkLink />
          <p className="mt-3 max-w-xs text-[13px] leading-relaxed text-muted">
            Utility payments settled in Zcash. Built for people who would rather not hand over
            identity documents to top up a phone.
          </p>
        </div>
        {COLUMNS.map((column) => (
          <div key={column.title}>
            <p className="label-xs text-muted">{column.title}</p>
            <ul className="mt-3 space-y-2">
              {column.links.map((link) => (
                <li key={link.href}>
                  <Link
                    href={link.href}
                    className="inline-block text-[13px] text-ink-2 transition-[color,transform] duration-150 hover:translate-x-0.5 hover:text-ink"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
      <div className="border-t border-line">
        <div className="mx-auto flex max-w-6xl flex-col gap-2 px-5 py-5 text-[12px] text-muted sm:flex-row sm:items-center sm:justify-between">
          <p>zutility. Operated from Lagos, Nigeria.</p>
          <p className="num">Zcash is a separate network. Rates move.</p>
        </div>
      </div>
    </footer>
  )
}
