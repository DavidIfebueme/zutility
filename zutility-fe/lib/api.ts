const apiBase = process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, '') || 'http://127.0.0.1:3001'

const wsBase = process.env.NEXT_PUBLIC_WS_URL?.replace(/\/$/, '') || apiBase.replace(/^http/, 'ws')

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

async function handleResponse<T>(response: Response, originalRequest: () => Promise<Response>): Promise<T> {
  if (response.status === 401) {
    const refreshRes = await fetch(`${apiBase}/api/v1/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
    })
    if (refreshRes.ok) {
      const retryRes = await originalRequest()
      if (!retryRes.ok) {
        throw new Error(`Request failed with status ${retryRes.status}`)
      }
      return retryRes.json() as Promise<T>
    }
    if (typeof window !== 'undefined') {
      const { useAuthStore } = require('@/store/auth')
      useAuthStore.getState().logout()
      window.location.href = '/login'
    }
    throw new Error('Session expired')
  }
  if (!response.ok) {
    throw new Error(`Request failed with status ${response.status}`)
  }
  return response.json() as Promise<T>
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
