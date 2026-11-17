import { NextResponse } from "next/server"
import type { NextRequest } from "next/server"
import { CURRENCY_COOKIE, currencyFromCountry, isCurrencyCode } from "@/lib/currency"

const APP_ROUTES = [
  "/dashboard",
  "/pay",
  "/gifts",
  "/subscriptions",
  "/history",
  "/settings",
  "/otc",
  "/p2p",
]

const CURRENCY_ROUTES = ["/", "/signup", "/login", "/gift"]
const COUNTRY_HEADERS = ["x-vercel-ip-country", "cf-ipcountry", "x-geo-country"]
const YEAR_SECONDS = 60 * 60 * 24 * 365

function matchesRoute(pathname: string, routes: string[]): boolean {
  return routes.some(
    (route) => pathname === route || (route !== "/" && pathname.startsWith(`${route}/`))
  )
}

function authGuard(request: NextRequest) {
  const signedIn =
    request.cookies.get("session")?.value || request.cookies.get("csrf_token")?.value

  if (signedIn) {
    return NextResponse.next()
  }

  const loginUrl = new URL("/login", request.url)
  loginUrl.searchParams.set("next", request.nextUrl.pathname + request.nextUrl.search)
  return NextResponse.redirect(loginUrl)
}

function currencyBranch(request: NextRequest) {
  const response = NextResponse.next()
  const current = request.cookies.get(CURRENCY_COOKIE)?.value
  if (current && isCurrencyCode(current)) {
    return response
  }

  const country = COUNTRY_HEADERS.map((name) => request.headers.get(name)).find(
    (value): value is string => Boolean(value)
  )
  const currency = country ? currencyFromCountry(country) : null
  if (!currency) {
    return response
  }

  response.cookies.set({
    name: CURRENCY_COOKIE,
    value: currency,
    path: "/",
    maxAge: YEAR_SECONDS,
    sameSite: "lax",
  })
  return response
}

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl
  if (matchesRoute(pathname, APP_ROUTES)) return authGuard(request)
  if (matchesRoute(pathname, CURRENCY_ROUTES)) return currencyBranch(request)
  return NextResponse.next()
}

export const config = {
  matcher: [
    "/dashboard/:path*",
    "/pay/:path*",
    "/gifts/:path*",
    "/subscriptions/:path*",
    "/history/:path*",
    "/settings/:path*",
    "/otc/:path*",
    "/p2p/:path*",
    "/",
    "/signup",
    "/login",
    "/gift/:path*",
  ],
}
