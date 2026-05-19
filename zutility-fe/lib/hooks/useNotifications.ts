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

  const fetchUnreadCount = React.useCallback(async () => {
    try {
      const res = await apiGet<{ count: number }>("/api/v1/notifications/unread-count")
      setUnreadCount(res.count)
    } catch {}
  }, [])

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
    fetchUnreadCount()
    fetchNotifications().finally(() => setLoading(false))
    const interval = setInterval(fetchUnreadCount, 30000)
    return () => clearInterval(interval)
  }, [fetchUnreadCount, fetchNotifications])

  return { notifications, unreadCount, loading, markRead, markAllRead, refresh: fetchNotifications }
}
