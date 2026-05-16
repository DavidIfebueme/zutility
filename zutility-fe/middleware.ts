import { NextResponse } from "next/server"
import type { NextRequest } from "next/server"

const PUBLIC_PATHS = ["/login", "/signup", "/verify", "/forgot-password", "/reset-password"]
const AUTH_PATHS = ["/login", "/signup"]

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl
  const hasAuthCookie = request.cookies.get("access_token")?.value

  if (pathname.startsWith("/dashboard") || pathname.startsWith("/pay") || pathname.startsWith("/history") || pathname.startsWith("/settings")) {
    if (!hasAuthCookie) {
      const loginUrl = new URL("/login", request.url)
      loginUrl.searchParams.set("next", pathname)
      return NextResponse.redirect(loginUrl)
    }
  }

  if (AUTH_PATHS.some(p => pathname.startsWith(p))) {
    if (hasAuthCookie) {
      return NextResponse.redirect(new URL("/dashboard", request.url))
    }
  }

  return NextResponse.next()
}

export const config = {
  matcher: ["/dashboard/:path*", "/pay/:path*", "/history/:path*", "/settings/:path*", "/login", "/signup"],
}
