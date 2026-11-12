# api reference

base url: `https://api.zutility.xyz`

## auth

| method | path | description |
|--------|------|-------------|
| POST | `/api/v1/auth/register` | register (currently closed) |
| POST | `/api/v1/auth/login` | login |
| POST | `/api/v1/auth/refresh` | refresh access token |
| POST | `/api/v1/auth/logout` | logout |
| GET | `/api/v1/auth/me` | get current user |
| POST | `/api/v1/auth/verify-email` | verify email |
| POST | `/api/v1/auth/resend-verification` | resend verification email |
| POST | `/api/v1/auth/forgot-password` | request password reset |
| POST | `/api/v1/auth/reset-password` | reset password |

## orders

| method | path | description |
|--------|------|-------------|
| POST | `/api/v1/orders` | create order |
| GET | `/api/v1/orders/:id` | get order by id |
| GET | `/api/v1/orders` | list order history |
| POST | `/api/v1/orders/:id/access-token` | issue a fresh order token for the owner |

## gifts

Prepaid utility gifts. The sender pays up front. The gift sits in `awaiting_claim` until the recipient enters their own account through the public link. The first claim wins.

| method | path | description |
|--------|------|-------------|
| POST | `/api/v1/gifts` | create a gift and its order |
| GET | `/api/v1/gifts` | list gifts sent by the current user |
| GET | `/api/v1/gifts/:code` | public gift preview and status |
| POST | `/api/v1/gifts/:code/claim` | claim a gift with the recipient account |
| POST | `/api/v1/gifts/:id/rotate-code` | issue a fresh gift link |
| POST | `/api/v1/gifts/:id/cancel` | cancel an unpaid gift |

Limits: 100,000 naira per gift, 500,000 naira outstanding per sender, claim window from 12 hours to 7 days. School fees are excluded. Unclaimed gifts deliver to the sender's backup account, or go to review when no backup was set.

## utilities

| method | path | description |
|--------|------|-------------|
| GET | `/api/v1/utilities` | list available utilities |
| GET | `/api/v1/utilities/:id/variations` | get utility variations |

## notifications

| method | path | description |
|--------|------|-------------|
| GET | `/api/v1/notifications` | list notifications |
| GET | `/api/v1/notifications/unread-count` | get unread count |
| PATCH | `/api/v1/notifications/:id/read` | mark as read |
| POST | `/api/v1/notifications/read-all` | mark all as read |

## settings

| method | path | description |
|--------|------|-------------|
| PATCH | `/api/v1/settings/profile` | update profile |
| POST | `/api/v1/settings/change-password` | change password |
| DELETE | `/api/v1/settings/account` | delete account |

## rates

| method | path | description |
|--------|------|-------------|
| GET | `/api/v1/rates` | get current zec rates |

## waitlist

| method | path | description |
|--------|------|-------------|
| POST | `/api/v1/waitlist/join` | join waitlist |
| POST | `/api/v1/waitlist/verify` | verify waitlist email |
| POST | `/api/v1/waitlist/resend` | resend verification |

## admin

| method | path | description |
|--------|------|-------------|
| GET | `/api/ops/admin/wallet/balance` | wallet balance (requires x-admin-secret) |

## authentication

access token is sent as http-only cookie (`access_token`, path `/api`). csrf protection via double-submit (`csrf_token` cookie embedded in jwt claims + non-httponly cookie).
