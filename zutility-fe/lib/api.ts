const apiBase = process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, '') || 'http://127.0.0.1:3001'

const wsBase = process.env.NEXT_PUBLIC_WS_URL?.replace(/\/$/, '') || apiBase.replace(/^http/, 'ws')

const REFRESH_STAMP_KEY = 'zutility-refresh-at'
const REFRESH_LEASE_KEY = 'zutility-refresh-lease'
const LEASE_MS = 8_000
const BACKOFF_MS = [300, 900, 2700]
const PROACTIVE_AFTER_MS = 10 * 60 * 1000

type RefreshOutcome = 'ok' | 'unauthenticated' | 'transient'

let refreshInFlight: Promise<RefreshOutcome> | null = null

function readStore(key: string): string | null {
  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

function writeStore(key: string, value: string | null) {
  try {
    if (value === null) {
      window.localStorage.removeItem(key)
    } else {
      window.localStorage.setItem(key, value)
    }
  } catch {}
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function refreshedSince(since: number): boolean {
  const stamp = Number(readStore(REFRESH_STAMP_KEY) || 0)
  return Number.isFinite(stamp) && stamp > since
}

function markRefreshed() {
  writeStore(REFRESH_STAMP_KEY, String(Date.now()))
}

export function markSessionFresh() {
  markRefreshed()
}

export function clearSessionFreshness() {
  writeStore(REFRESH_STAMP_KEY, null)
  writeStore(REFRESH_LEASE_KEY, null)
}

function claimLease(): string | null {
  const token = `${Date.now()}:${Math.random()}`
  const now = Date.now()
  const until = Number(readStore(REFRESH_LEASE_KEY)?.split(':')[0] || 0)
  if (Number.isFinite(until) && until > now) return null

  writeStore(REFRESH_LEASE_KEY, `${now + LEASE_MS}:${token}`)
  const stored = readStore(REFRESH_LEASE_KEY)
  if (stored && stored.includes(token)) return token
  return null
}

function releaseLease(token: string) {
  const stored = readStore(REFRESH_LEASE_KEY)
  if (stored && stored.includes(token)) {
    writeStore(REFRESH_LEASE_KEY, null)
  }
}

async function refreshOnce(): Promise<RefreshOutcome> {
  let response: Response
  try {
    response = await fetch(`${apiBase}/api/v1/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
    })
  } catch {
    return 'transient'
  }

  if (response.ok) {
    markRefreshed()
    return 'ok'
  }
  if (response.status === 401) {
    return 'unauthenticated'
  }
  return 'transient'
}

async function runRefresh(): Promise<RefreshOutcome> {
  const startedAt = Date.now()
  if (refreshedSince(startedAt)) return 'ok'

  for (const wait of [0, ...BACKOFF_MS]) {
    if (wait > 0) await sleep(wait)
    if (refreshedSince(startedAt)) return 'ok'

    const lease = claimLease()
    if (!lease) {
      for (let attempt = 0; attempt < 10; attempt++) {
        await sleep(300)
        if (refreshedSince(startedAt)) return 'ok'
      }
      continue
    }

    try {
      const outcome = await refreshOnce()
      if (outcome !== 'transient') return outcome
    } finally {
      releaseLease(lease)
    }
  }

  return 'transient'
}

function refreshSession(): Promise<RefreshOutcome> {
  if (!refreshInFlight) {
    refreshInFlight = runRefresh().finally(() => {
      refreshInFlight = null
    })
  }
  return refreshInFlight
}

export function getApiBaseUrl(): string {
  return apiBase
}

export function getWsBaseUrl(): string {
  return wsBase
}

function getAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
  }
  const csrf = getCsrfToken()
  if (csrf) {
    headers['x-csrf-token'] = csrf
  }
  return headers
}

function getCsrfToken(): string | null {
  if (typeof document === 'undefined') return null
  return document.cookie
    .split('; ')
    .find(row => row.startsWith('csrf_token='))
    ?.split('=')[1] ?? null
}

async function endSession() {
  if (typeof window === 'undefined') return
  try {
    await fetch(`${apiBase}/api/v1/auth/logout`, {
      method: 'POST',
      credentials: 'include',
      headers: getAuthHeaders(),
    })
  } catch {}

  const { useAuthStore } = require('@/store/auth')
  const { useOrderStore } = require('@/store/order')
  useAuthStore.setState({ user: null, isAuthenticated: false, preferredCurrency: null })
  useOrderStore.getState().clearActiveOrder()
  writeStore(REFRESH_STAMP_KEY, null)
  writeStore(REFRESH_LEASE_KEY, null)
  const next = encodeURIComponent(window.location.pathname + window.location.search)
  window.location.assign(`/login?next=${next}`)
}

async function handleResponse<T>(response: Response, originalRequest: () => Promise<Response>): Promise<T> {
  if (response.status === 401) {
    const outcome = await refreshSession()

    if (outcome === 'ok') {
      const retryRes = await originalRequest()
      if (retryRes.status === 401) {
        await endSession()
        throw new Error('Session expired')
      }
      if (!retryRes.ok) {
        throw new Error(`Request failed with status ${retryRes.status}`)
      }
      return retryRes.json() as Promise<T>
    }

    if (outcome === 'unauthenticated') {
      await endSession()
      throw new Error('Session expired')
    }

    throw new Error('We could not reach the server. Your session is still active, so try again.')
  }

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`
    try {
      const body: { error?: unknown } = await response.json()
      if (typeof body.error === "string") message = body.error
    } catch {
      message = `Request failed with status ${response.status}`
    }
    throw new Error(message)
  }
  return response.json() as Promise<T>
}

export async function ensureFreshSession(): Promise<void> {
  if (typeof window === 'undefined') return
  const stamp = Number(readStore(REFRESH_STAMP_KEY) || 0)
  if (Date.now() - stamp < PROACTIVE_AFTER_MS) return

  const outcome = await refreshSession()
  if (outcome === 'unauthenticated') {
    await endSession()
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  const doRequest = () => fetch(`${apiBase}${path}`, {
    method: 'GET',
    headers: getAuthHeaders(),
    credentials: 'include',
    cache: 'no-store',
  })
  const response = await doRequest()
  return handleResponse<T>(response, doRequest)
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const doRequest = () => fetch(`${apiBase}${path}`, {
    method: 'POST',
    headers: getAuthHeaders(),
    credentials: 'include',
    body: JSON.stringify(body),
  })
  const response = await doRequest()
  return handleResponse<T>(response, doRequest)
}

export async function apiPostRaw(path: string, body: unknown): Promise<Response> {
  return fetch(`${apiBase}${path}`, {
    method: 'POST',
    headers: getAuthHeaders(),
    credentials: 'include',
    body: JSON.stringify(body),
  })
}
