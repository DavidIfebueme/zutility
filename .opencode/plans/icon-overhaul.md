# Icon Overhaul Plan: itshover Animated Icons + Custom Brand Icons

## Summary
Replace ALL lucide-react icons across the entire zutility-fe app with:
1. **itshover animated icons** — for generic UI icons (nav, actions, status, form inputs)
2. **Custom animated brand icons** — for Nigerian utility brands (MTN, Glo, DSTV, DISCOs, etc.)
3. **Same motion/react pattern** — all icons animate on hover, consistent API

## Architecture

### Directory Structure
```
components/icons/
  types.ts                    # Shared AnimatedIconProps / AnimatedIconHandle types
  # itshover icons (downloaded from GitHub, MIT/Apache-2.0)
  home-icon.tsx
  credit-card.tsx
  arrow-narrow-right-icon.tsx
  arrow-back-icon.tsx
  arrow-back-up-icon.tsx
  history-circle-icon.tsx
  gear-icon.tsx
  logout-icon.tsx
  filled-bell-icon.tsx
  checked-icon.tsx
  triangle-alert-icon.tsx     # custom (not in itshover)
  clock-icon.tsx
  shield-check.tsx
  phone-volume.tsx
  wifi-icon.tsx
  book-icon.tsx
  plug-connected-icon.tsx
  lock-icon.tsx
  mail-filled-icon.tsx
  user-icon.tsx
  copy-icon.tsx
  trash-icon.tsx
  globe-icon.tsx
  send-horizontal-icon.tsx
  send-icon.tsx
  down-chevron.tsx
  right-chevron.tsx
  x-icon.tsx
  wallet-icon.tsx
  shopping-cart-icon.tsx
  qrcode-icon.tsx             # custom (not in itshover)
  refresh-icon.tsx
  magnifier-icon.tsx
  info-circle-icon.tsx
  double-check-icon.tsx
  message-circle-icon.tsx
  users-group-icon.tsx
  eye-icon.tsx
  eye-off-icon.tsx
  link-icon.tsx
  map-pin-icon.tsx
  tv-icon.tsx                 # custom (not in itshover)
  timer-off-icon.tsx          # custom (not in itshover)
  menu-icon.tsx               # custom hamburger (keep simple)
  # Custom brand icons (original SVGs, animated with same pattern)
  brands/
    mtn-icon.tsx
    airtel-icon.tsx
    glo-icon.tsx
    9mobile-icon.tsx
    dstv-icon.tsx
    gotv-icon.tsx
    startimes-icon.tsx
    showmax-icon.tsx
    ikeja-electric-icon.tsx
    eko-electric-icon.tsx
    abuja-electric-icon.tsx
    ibadan-electric-icon.tsx
    kano-electric-icon.tsx
    phed-electric-icon.tsx
    jos-electric-icon.tsx
    kaduna-electric-icon.tsx
    enugu-electric-icon.tsx
    benin-electric-icon.tsx
    yola-electric-icon.tsx
    aba-electric-icon.tsx
    waec-icon.tsx
    jamb-icon.tsx
    school-fees-icon.tsx
```

### Icon Component Pattern (all icons follow this)
```tsx
import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const MyIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();
    const start = useCallback(async () => { /* hover animation */ }, [animate]);
    const stop = useCallback(() => { /* reset */ }, [animate]);
    useImperativeHandle(ref, () => ({ startAnimation: start, stopAnimation: stop }));
    return (
      <motion.svg ref={scope} onHoverStart={start} onHoverEnd={stop}
        xmlns="http://www.w3.org/2000/svg" width={size} height={size}
        viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={strokeWidth}
        strokeLinecap="round" strokeLinejoin="round"
        className={`cursor-pointer ${className}`}>
        <motion.path ... />
      </motion.svg>
    );
  }
);
MyIcon.displayName = "MyIcon";
export default MyIcon;
```

### Brand Icon SVG Sources
- **MTN, Airtel, Glo** → Simple Icons (simpleicons.org) - MIT license
- **DSTV (MultiChoice)** → Simple Icons
- **GOtv** → Simple Icons or derived from MultiChoice
- **Startimes** → Simple Icons
- **Showmax** → Simple Icons or derived from MultiChoice
- **9mobile** → Create stylized "9e" lettermark SVG
- **DISCOs (12)** → Stylized lightning bolt (`<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>`) with initial letter badge
- **WAEC** → Create book + "W" mark
- **JAMB** → Create document + "J" mark  
- **School Fees** → Graduation cap or building SVG

### DISCO Lightning Bolt Animation
Each DISCO icon animates on hover:
1. Lightning bolt pulses/brightens (opacity 0.7→1)
2. Subtle scale bounce (1→1.1→1)
3. Initial letter (I, E, A, IB, K, P, J, Kd, En, B, Y, AB) slides up slightly

## File-by-File Changes

### 1. lib/constants.ts
- Change `iconType` field from generic strings to specific icon component names
- Add `iconComponent` or `brandId` field to each utility entry
- Update landing page filter indices to include School Fees (index 27)
- UTILITY_CATEGORIES get proper icon references

### 2. app/(app)/layout.tsx
| Line | Old (lucide) | New (itshover) |
|------|-------------|----------------|
| Home | `Home` | `HomeIcon` |
| CreditCard | `CreditCard` | `CreditCard` |
| ArrowRightLeft | `ArrowRightLeft` | `ArrowBackUpIcon` |
| Store | `Store` | `ShoppingCartIcon` |
| History | `History` | `HistoryCircleIcon` |
| Settings | `Settings` | `GearIcon` |
| LogOut | `LogOut` | `LogoutIcon` |
| Menu | `Menu` | keep lucide Menu (hamburger, no animation needed) |

### 3. components/ui/notification-dropdown.tsx
| Old | New |
|-----|-----|
| Bell | FilledBellIcon |
| CheckCircle2 | CheckedIcon |
| AlertCircle | TriangleAlertIcon |
| Clock | ClockIcon |
| Loader2 | keep (spinner) |
| XCircle | XIcon + circle wrapper |
| AlertTriangle | TriangleAlertIcon |
| Shield | ShieldCheck |
| X | XIcon |
| CheckCheck | DoubleCheckIcon |

### 4. app/(app)/dashboard/page.tsx
| Old | New |
|-----|-----|
| ArrowRight | ArrowNarrowRightIcon |
| CreditCard | CreditCardIcon |
| ArrowRightLeft | ArrowBackUpIcon |
| Store | ShoppingCartIcon |
| Loader2 | keep |
| CheckCircle2 | CheckedIcon |
| AlertCircle | TriangleAlertIcon |
| Clock | ClockIcon |
| History | HistoryCircleIcon |

### 5. app/(app)/pay/page.tsx (BIGGEST CHANGE)
- Replace `getIconForType()` function to return itshover icons for categories
- Replace utility selection card icons: each utility gets its **brand icon**
- New import map for brand icons by utility ID
| Category | New Icon |
|----------|----------|
| airtime | PhoneVolume (category) + brand icons per provider (MTN, Airtel, Glo, 9mobile) |
| data | WifiIcon (category) + brand icons per provider |
| tv | TvIcon (custom) + brand icons (DSTV, GOtv, Startimes, Showmax) |
| electricity | PlugConnectedIcon (category) + DISCO bolt+initial icons |
| education | BookIcon (category) + WAEC/JAMB brand icons |
| school | BookIcon/SchoolFeesIcon |

### 6. app/(app)/pay/[orderId]/page.tsx
| Old | New |
|-----|-----|
| Copy | CopyIcon |
| CheckCircle2 | CheckedIcon |
| AlertCircle | TriangleAlertIcon |
| Clock | ClockIcon |
| ArrowLeft | ArrowBackIcon |
| Zap | PlugConnectedIcon |
| XCircle | XIcon |
| TimerOff | TimerOffIcon (custom) |

### 7. app/(app)/settings/page.tsx
| Old | New |
|-----|-----|
| User | UserIcon |
| Mail | MailFilledIcon |
| CheckCircle2 | CheckedIcon |
| Lock | LockIcon |
| Shield | ShieldCheck |
| Globe | GlobeIcon |
| Trash2 | TrashIcon |
| AlertTriangle | TriangleAlertIcon |
| ChevronRight | RightChevron |

### 8. app/(app)/history/page.tsx
| Old | New |
|-----|-----|
| Clock | ClockIcon |
| CheckCircle2 | CheckedIcon |
| AlertCircle | TriangleAlertIcon |
| History | HistoryCircleIcon |
| Loader2 | keep |
| ArrowRight | ArrowNarrowRightIcon |

### 9. app/marketing/page.tsx (LANDING PAGE - KEY VISUAL CHANGE)
- Product cards: Zap→PlugConnectedIcon, ArrowRightLeft→ArrowBackUpIcon, Store→ShoppingCartIcon
- **Supported Utilities grid**: Replace `{u.name.charAt(0)}` letter initials with actual `<BrandIcon>` components
- Add School Fees card to the grid (currently missing)
- Grid shows: MTN, MTN Data, DSTV, Ikeja Electric, JAMB Pin, School Fees (+ maybe one more)

### 10. app/marketing/how-it-works/page.tsx
Step icons: Shield→ShieldCheck, CheckCircle2→CheckedIcon, QrCode→QrcodeIcon, Zap→PlugConnectedIcon, Lock→LockIcon, Clock→ClockIcon, ArrowRight→ArrowNarrowRightIcon

### 11. app/marketing/support/page.tsx
Mail→MailFilledIcon, MessageSquare→MessageCircleIcon, User→UserIcon, Send→SendIcon, CheckCircle2→CheckedIcon, ArrowRight→ArrowNarrowRightIcon

### 12. app/marketing/waitlist/page.tsx
Mail→MailFilledIcon, User→UserIcon, Shield→ShieldCheck, Zap→PlugConnectedIcon, Clock→ClockIcon, Users→UsersGroupIcon, Check→CheckedIcon, Copy→CopyIcon, ArrowRight→ArrowNarrowRightIcon

### 13. Auth pages
- login: Mail→MailFilledIcon, Lock→LockIcon, ArrowRight→ArrowNarrowRightIcon
- signup: Lock→LockIcon
- forgot-password: Mail→MailFilledIcon, ArrowLeft→ArrowBackIcon, ArrowRight→ArrowNarrowRightIcon
- reset-password: Lock→LockIcon, ArrowRight→ArrowNarrowRightIcon
- verify: Mail→MailFilledIcon, ArrowRight→ArrowNarrowRightIcon, RefreshCw→RefreshIcon

### 14. UI primitives
- button.tsx: Loader2 → keep (spinner animation is different paradigm)
- modal.tsx: X → XIcon
- stepper.tsx: Check → CheckedIcon
- copy-field.tsx: Check→CheckedIcon, Copy→CopyIcon

## Execution Order
1. Create `components/icons/types.ts`
2. Download ~40 itshover icon files from GitHub raw
3. Create ~6 custom generic icons (triangle-alert, qrcode, tv, timer-off, menu)
4. Create ~23 brand icon components in `components/icons/brands/`
5. Update `lib/constants.ts`
6. Update files in dependency order: UI primitives → layout → pages (bottom-up)

## Verification
- `npx tsc --noEmit` passes
- `npx next lint` passes (or only pre-existing warnings)
- Visual check: landing page shows real brand logos, not letters
- Hover over any icon triggers smooth animation
