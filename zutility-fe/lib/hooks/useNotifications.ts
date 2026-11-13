"use client"

import * as React from "react"
import { apiGet, apiPost } from "@/lib/api"

export interface Notification {
  id: string
  order_id: string | null
  type: string
  title: string
  body: string
  detail: Record<string, unknown>
  read: boolean
  created_at: string
}

export function useNotifications() {
  const [notifications, setNotifications] = React.useState<Notification[]>([])
  const [unreadCount, setUnreadCount] = React.useState(0)
  const [loading, setLoading] = React.useState(true)

  const fetchNotifications = React.useCallback(async () => {
    try {
      const res = await apiGet<Notification[]>("/api/v1/notifications")
      setNotifications(res)
    } catch {}
  }, [])

  const markRead = React.useCallback(async (id: string) => {
    try {
      await apiPost(`/api/v1/notifications/${id}/read`, {})
      setNotifications((prev) =>
        prev.map((n) => (n.id === id ? { ...n, read: true } : n))
      )
      setUnreadCount((prev) => Math.max(0, prev - 1))
    } catch {}
  }, [])

  const markAllRead = React.useCallback(async () => {
    try {
      await apiPost("/api/v1/notifications/mark-all-read", {})
      setNotifications((prev) => prev.map((n) => ({ ...n, read: true })))
      setUnreadCount(0)
    } catch {}
  }, [])

  React.useEffect(() => {
    let active = true

    const poll = async () => {
      const count = await apiGet<{ count: number }>("/api/v1/notifications/unread-count").catch(
        () => null
      )
      if (!active) return
      if (count) setUnreadCount(count.count)
      setLoading(false)
    }

    void poll()
    const interval = setInterval(poll, 30000)
    return () => {
      active = false
      clearInterval(interval)
    }
  }, [])

  return { notifications, unreadCount, loading, markRead, markAllRead, refresh: fetchNotifications }
}
