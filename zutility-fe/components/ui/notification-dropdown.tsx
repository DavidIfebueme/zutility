"use client"

import * as React from "react"
import Link from "next/link"
import { motion, AnimatePresence } from "motion/react"
import FilledBellIcon from "@/components/icons/filled-bell-icon"
import CheckedIcon from "@/components/icons/checked-icon"
import TriangleAlertIcon from "@/components/icons/triangle-alert-icon"
import ClockIcon from "@/components/icons/clock-icon"
import XIcon from "@/components/icons/x-icon"
import DoubleCheckIcon from "@/components/icons/double-check-icon"
import { Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
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

function getNotificationIcon(type: string) {
  switch (type) {
    case "payment_detected":
      return <ClockIcon size={16} className="text-accent-zec" />
    case "payment_confirmed":
      return <CheckedIcon size={16} className="text-accent-green" />
    case "utility_dispatching":
      return <Loader2 className="h-4 w-4 text-accent-zec animate-spin" />
    case "order_completed":
      return <CheckedIcon size={16} className="text-accent-green" />
    case "order_failed":
      return <XIcon size={16} className="text-accent-red" />
    case "order_expired":
      return <TriangleAlertIcon size={16} className="text-text-muted" />
    case "order_flagged":
      return <TriangleAlertIcon size={16} className="text-accent-zec" />
    default:
      return <FilledBellIcon size={16} className="text-text-muted" />
  }
}

function NotificationItem({
  notification,
  onMarkRead,
}: {
  notification: Notification
  onMarkRead: (id: string) => void
}) {
  const href = notification.order_id ? `/pay/${notification.order_id}` : null

  const content = (
    <div
      className={cn(
        "flex items-start gap-3 p-3 transition-colors cursor-pointer",
        notification.read ? "bg-bg-surface" : "bg-bg-elevated",
        href && "hover:bg-bg-elevated"
      )}
      onClick={() => {
        if (!notification.read) onMarkRead(notification.id)
      }}
    >
      <div className="mt-0.5 shrink-0">{getNotificationIcon(notification.type)}</div>
      <div className="flex-1 min-w-0">
        <p className={cn("text-sm leading-snug", notification.read ? "text-text-secondary" : "text-text-primary font-medium")}>
          {notification.title}
        </p>
        <p className="text-xs text-text-muted mt-0.5 line-clamp-2">{notification.body}</p>
        <p className="text-[10px] text-text-muted mt-1">{timeAgo(notification.created_at)}</p>
      </div>
      {!notification.read && (
        <div className="mt-1.5 h-2 w-2 rounded-full bg-accent-zec shrink-0" />
      )}
    </div>
  )

  if (href) {
    return <Link href={href}>{content}</Link>
  }
  return content
}

export function NotificationDropdown() {
  const { notifications, unreadCount, loading, markRead, markAllRead } = useNotifications()
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
        onClick={() => setIsOpen(!isOpen)}
        className="relative text-text-secondary hover:text-text-primary transition-colors"
        aria-label="Notifications"
      >
        <FilledBellIcon size={20} />
        {unreadCount > 0 && (
          <span className="absolute -top-1.5 -right-1.5 min-w-[18px] h-[18px] flex items-center justify-center rounded-full bg-accent-red text-white text-[10px] font-bold px-1">
            {unreadCount > 99 ? "99+" : unreadCount}
          </span>
        )}
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: -8, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className="absolute right-0 top-full mt-2 w-80 sm:w-96 rounded-xl border border-border-subtle bg-bg-surface shadow-xl z-50 overflow-hidden"
          >
            <div className="flex items-center justify-between px-4 py-3 border-b border-border-subtle">
              <h3 className="text-sm font-semibold">Notifications</h3>
              <div className="flex items-center gap-2">
                {unreadCount > 0 && (
                  <button
                    onClick={markAllRead}
                    className="text-xs text-accent-zec hover:underline flex items-center gap-1"
                  >
                    <DoubleCheckIcon size={14} />
                    Mark all read
                  </button>
                )}
                <button
                  onClick={() => setIsOpen(false)}
                  className="text-text-muted hover:text-text-primary"
                >
                  <XIcon size={16} />
                </button>
              </div>
            </div>

            <div className="max-h-96 overflow-y-auto">
              {loading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-5 w-5 animate-spin text-text-muted" />
                </div>
              ) : notifications.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12 text-center">
                  <FilledBellIcon size={32} className="text-text-muted mb-2" />
                  <p className="text-sm text-text-secondary">No notifications yet</p>
                  <p className="text-xs text-text-muted mt-1">We'll notify you about your orders</p>
                </div>
              ) : (
                <div className="divide-y divide-border-subtle">
                  {notifications.map((n) => (
                    <NotificationItem key={n.id} notification={n} onMarkRead={markRead} />
                  ))}
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}
