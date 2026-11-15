import { NextResponse } from "next/server"
import type { NextRequest } from "next/server"

export function middleware(request: NextRequest) {
  const signedIn =
    request.cookies.get("session")?.value || request.cookies.get("csrf_token")?.value

  if (signedIn) {
    return NextResponse.next()
  }

  const loginUrl = new URL("/login", request.url)
  loginUrl.searchParams.set("next", request.nextUrl.pathname + request.nextUrl.search)
  return NextResponse.redirect(loginUrl)
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
  ],
}
