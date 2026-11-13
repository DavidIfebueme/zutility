"use client"

import * as React from "react"
import Link from "next/link"
import { Bell, BellOff, Check, Clock, Loader2, TriangleAlert, X } from "lucide-react"
import { useNotifications, type Notification } from "@/lib/hooks/useNotifications"
import { cn } from "@/lib/utils"

function timeAgo(dateStr: string): string {
  const seconds = Math.floor((Date.now() - new Date(dateStr).getTime()) / 1000)
  if (seconds < 60) return "just now"
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  if (days < 7) return `${days}d ago`
  return new Date(dateStr).toLocaleDateString()
}

function NotificationIcon({ type }: { type: string }) {
  const className = "h-3.5 w-3.5"
  switch (type) {
    case "payment_detected":
      return <Clock className={cn(className, "text-gold-deep")} />
    case "payment_confirmed":
    case "order_completed":
      return <Check className={cn(className, "text-green")} />
    case "utility_dispatching":
      return <Loader2 className={cn(className, "animate-spin text-gold-deep")} />
    case "order_failed":
      return <X className={cn(className, "text-rust")} />
    case "order_expired":
    case "order_flagged":
      return <TriangleAlert className={cn(className, "text-gold-deep")} />
    default:
      return <Bell className={cn(className, "text-muted")} />
  }
}

function NotificationRow({
  notification,
  onMarkRead,
}: {
  notification: Notification
  onMarkRead: (id: string) => void
}) {
  const href = notification.order_id ? `/pay/${notification.order_id}` : null

  const body = (
    <div
      className={cn(
        "flex items-start gap-2.5 px-4 py-3 transition-colors hover:bg-paper-2",
        notification.read ? "bg-cream" : "bg-paper"
      )}
      onClick={() => {
        if (!notification.read) onMarkRead(notification.id)
      }}
    >
      <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-line">
        <NotificationIcon type={notification.type} />
      </span>
      <div className="min-w-0 flex-1">
        <p
          className={cn(
            "text-[13px] leading-snug",
            notification.read ? "text-ink-2" : "font-medium text-ink"
          )}
        >
          {notification.title}
        </p>
        <p className="mt-0.5 line-clamp-2 text-[12px] text-muted">{notification.body}</p>
        <p className="mt-1 text-[11px] text-muted">{timeAgo(notification.created_at)}</p>
      </div>
      {!notification.read && <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-gold" />}
    </div>
  )

  if (href) return <Link href={href}>{body}</Link>
  return body
}

export function NotificationDropdown() {
  const { notifications, unreadCount, loading, markRead, markAllRead, refresh } = useNotifications()
  const [isOpen, setIsOpen] = React.useState(false)
  const dropdownRef = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false)
      }
    }
    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [])

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        onClick={() => {
          const next = !isOpen
          setIsOpen(next)
          if (next) void refresh()
        }}
        className="relative flex h-8 w-8 items-center justify-center rounded-md text-muted transition-colors hover:bg-cream hover:text-ink"
        aria-label="Notifications"
      >
        <Bell className="h-4 w-4" strokeWidth={1.7} />
        {unreadCount > 0 && (
          <span className="absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-gold px-1 text-[9px] font-semibold text-ink">
            {unreadCount > 99 ? "99+" : unreadCount}
          </span>
        )}
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full z-50 mt-2 w-80 overflow-hidden rounded-lg border border-line bg-cream shadow-[0_8px_24px_rgba(20,19,15,0.12)] sm:w-96">
          <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
            <h3 className="text-[13px] font-medium text-ink">Notifications</h3>
            <div className="flex items-center gap-2">
              {unreadCount > 0 && (
                <button
                  onClick={markAllRead}
                  className="text-[11px] text-gold-deep u-link"
                >
                  Mark all read
                </button>
              )}
              <button
                onClick={() => setIsOpen(false)}
                className="flex h-6 w-6 items-center justify-center rounded-md text-muted hover:text-ink"
                aria-label="Close notifications"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>

          <div className="max-h-96 overflow-y-auto">
            {loading ? (
              <div className="flex items-center justify-center py-10">
                <Loader2 className="h-4 w-4 animate-spin text-muted" />
              </div>
            ) : notifications.length === 0 ? (
              <div className="flex flex-col items-center justify-center px-4 py-10 text-center">
                <BellOff className="h-5 w-5 text-muted" strokeWidth={1.6} />
                <p className="mt-2 text-[13px] text-ink">Nothing yet</p>
                <p className="mt-1 text-[12px] text-muted">
                  Order updates appear here.
                </p>
              </div>
            ) : (
              <div className="divide-y divide-line">
                {notifications.map((notification) => (
                  <NotificationRow
                    key={notification.id}
                    notification={notification}
                    onMarkRead={markRead}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
