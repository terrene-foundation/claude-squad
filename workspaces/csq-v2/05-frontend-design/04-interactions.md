# csq v2.0 — Interaction Patterns

How the app responds to user input. Keyboard, mouse, drag, right-click, shortcuts.

## Keyboard Shortcuts

### Global (active in any window)

| Shortcut        | Action                       |
| --------------- | ---------------------------- |
| `⌘/Ctrl + 1..9` | Quick-switch to account 1-9  |
| `⌘/Ctrl + 0`    | Open account switcher        |
| `⌘/Ctrl + R`    | Refresh all tokens           |
| `⌘/Ctrl + ,`    | Open settings                |
| `⌘/Ctrl + F`    | Search / filter accounts     |
| `⌘/Ctrl + N`    | Add new account (OAuth flow) |
| `⌘/Ctrl + W`    | Close current window         |
| `⌘/Ctrl + Q`    | Quit application             |
| `Esc`           | Close modal / cancel action  |

Shortcuts use `⌘` on macOS, `Ctrl` on Windows/Linux. Display in menus and tooltips with the correct platform key.

### Dashboard (when grid has focus)

| Shortcut  | Action                                       |
| --------- | -------------------------------------------- |
| `↑ ↓ ← →` | Navigate between cards                       |
| `Enter`   | Open focused card's details                  |
| `Space`   | Activate focused card (swap to that account) |
| `Delete`  | Remove focused account (with confirmation)   |
| `/`       | Focus search bar                             |

### Modal Dialogs

| Shortcut            | Action                         |
| ------------------- | ------------------------------ |
| `Tab` / `Shift+Tab` | Move between fields            |
| `Enter`             | Submit (on the primary button) |
| `Esc`               | Cancel / close                 |

## Mouse Interactions

### Card Hover

- Background shifts from `--bg-surface` → `--bg-hover`
- Border shifts from `--border-subtle` → `--border-default`
- Action buttons fade in (opacity 0 → 1, 150ms)
- Cursor: `pointer` on the card, `default` on nested buttons

### Card Click

- Single click on card body → activate (swap to account)
- Single click on "Details" button → open detail view
- Double click on card → open detail view (alternative)
- Right click → context menu

### Context Menu (right click on card)

```
┌────────────────────────┐
│ Activate            ⏎  │
│ Details                │
│ ──────────────         │
│ Refresh Token          │
│ Re-login...            │
│ ──────────────         │
│ Copy email             │
│ Copy account number    │
│ ──────────────         │
│ Edit label...          │
│ Remove...              │
└────────────────────────┘
```

Built via Tauri `menu` API. Position at cursor click coordinates. Dismiss on click outside or `Esc`.

### Drag to Reorder

Users can drag account cards to change display order (persisted in settings).

- Mouse down on card (not on button) → card lifts (shadow increases, scale 1.02)
- Drag → card follows cursor with 50% opacity ghost
- Other cards shift to make space
- Mouse up → card drops into new position, animates into place (250ms ease-out)
- `Esc` during drag → cancel, card returns to original position

Use `@neodrag/svelte` or implement with pointer events directly. Update order via Tauri command `reorder_accounts(newOrder: number[])`.

### Hover Tooltips

Tooltips appear after 600ms hover, dismiss on mouseleave.

- Truncated text (email, labels) → show full text
- Icons without labels → show action description
- Timestamps → show exact ISO time ("2026-04-10 18:24:41 UTC")
- Status badges → show reason ("Token expires in 2h 14m")

Tooltip style: small dark rounded rect, 11px text, 6px padding, subtle shadow. Position above the element with 8px offset.

## Loading States

### Initial Load (app startup)

Before accounts are fetched from the daemon, show skeletons:

```
┌──────────────────────┐  ┌──────────────────────┐
│ ░░░░░░  ░░░░░░░░░░   │  │ ░░░░░░  ░░░░░░░░░░   │
│         ░░░░░        │  │         ░░░░░        │
│ ░░ ░░░░░░░░░░ ░░░░   │  │ ░░ ░░░░░░░░░░ ░░░░   │
│ ░░ ░░░░░░░░░░ ░░░░   │  │ ░░ ░░░░░░░░░░ ░░░░   │
└──────────────────────┘  └──────────────────────┘
```

Shimmer animation (defined in component library). Show up to 4 skeleton cards. Replace with real cards as they load.

### Action Loading

Buttons show inline spinner when an action is in progress:

```
[ Refreshing... ⟳ ]   (disabled, spinner rotates)
```

Don't block the whole UI — only the button that triggered the action. Other interactions remain responsive.

### Optimistic Updates

For fast operations (swap, local state), update UI immediately and reconcile on backend response. For slow operations (token refresh, OAuth), show explicit loading state.

| Action         | Strategy                                                |
| -------------- | ------------------------------------------------------- |
| Swap account   | Optimistic — UI switches immediately, rollback on error |
| Refresh token  | Explicit loading — show spinner on card                 |
| Add account    | Explicit loading — modal with progress steps            |
| Remove account | Optimistic — fade out immediately, undo toast for 5s    |
| Reorder cards  | Optimistic — animate immediately                        |

## Error Recovery

### Network Errors

Token refresh fails due to network:

1. Toast notification: "Couldn't reach Claude API. Check your connection."
2. Card shows `⚠ expiring` state
3. Auto-retry after 30s, 60s, 120s (exponential backoff, max 5 retries)
4. After max retries, show LOGIN-NEEDED state

### Auth Errors

Refresh token invalid (needs re-login):

1. Toast: "Account #3 needs to re-login."
2. Card shows LOGIN-NEEDED state with red border
3. Clicking the card opens the re-login flow directly (OAuth PKCE)

### Rate Limit Errors

Cannot switch because target is also rate-limited:

1. Toast: "Cannot switch to account #5 — quota at 100%. Try #1 or #2."
2. Don't perform the swap
3. Suggest an alternative in the toast (link to switch to suggested account)

### Concurrent Modification

User swapped via CLI while GUI was showing old state:

1. Detect via file watcher on `.current-account`
2. Refresh account list silently (no toast)
3. Update active account indicator with a smooth transition

## Search & Filter

### Search Bar

`⌘/Ctrl + F` opens a search bar above the grid:

```
[🔍 Filter accounts...                                 ×]
```

Filters in real-time (debounced 150ms). Matches against:

- Account number (exact or prefix)
- Email (substring, case-insensitive)
- Subscription tier
- Status labels

Empty query shows all accounts. Clear button (×) resets.

### Filter Chips

Below the search bar, show filter chips for common filters:

```
[ All ] [ Active ] [ Healthy ] [ Warning ] [ LOGIN-NEEDED ]
```

Single-select. Clicking a chip toggles that filter. Combined with search for narrow queries.

### No Results

When filters match nothing:

```
         No matching accounts

    No accounts match "admin@"
         [ Clear filters ]
```

## Toasts & Notifications

### Toast Variants

| Variant   | When                              | Auto-dismiss            |
| --------- | --------------------------------- | ----------------------- |
| `info`    | Neutral info (account reordered)  | 3s                      |
| `success` | Successful action (swap, refresh) | 3s                      |
| `warning` | Caution (quota > 90%)             | 5s                      |
| `error`   | Failure (refresh failed)          | Sticky (user dismisses) |

### Toast Actions

Toasts can have one action button:

```
┌───────────────────────────────────────┐
│ ✓ Swapped to #3 team@example.com      │
│   Token valid for 3h 28m              │
│                       [ Open Details ]│
└───────────────────────────────────────┘
```

Action triggers, then toast dismisses.

### Toast Stacking

Maximum 5 visible toasts. Older ones auto-dismiss when a 6th appears. Newest toasts appear at the top.

### Native Notifications

For events that happen while the app is minimized, also fire a native OS notification via Tauri:

- Account rotation (auto-rotate triggered)
- Token refresh failed (LOGIN-NEEDED)
- All accounts exhausted

Native notifications are muted if the user is actively using the window (focus detection).

## Drag and Drop (Credentials Import)

Users can drag a `.credentials.json` file onto the dashboard to import an account:

1. Drag enters window → show overlay "Drop to import credentials"
2. Drag leaves → hide overlay
3. Drop → parse file, validate, prompt for account number, import
4. Show success/error toast

Security: only accept files from the same filesystem (no network drops), validate JSON schema, reject if already imported.

## Accessibility Interactions

### Screen Reader Announcements

Use `aria-live` regions for dynamic content:

```html
<!-- Toast region -->
<div aria-live="polite" aria-atomic="false">
  {#each toasts as toast (toast.id)}
  <Toast {...toast} />
  {/each}
</div>

<!-- Quota updates -->
<div aria-live="polite" class="sr-only">
  {currentAccount ? `Account ${currentAccount.id},
  ${currentAccount.fiveHourPct}% used` : ''}
</div>
```

- `polite` for non-critical (toasts, quota updates)
- `assertive` only for errors that need immediate attention (LOGIN-NEEDED)

### Focus Management

- Opening a modal: move focus to the first interactive element
- Closing a modal: return focus to the element that triggered it
- After an action completes: keep focus on the action element (don't jump around)
- `Tab` order follows visual order (top to bottom, left to right)

### Keyboard-Only Navigation

All interactions MUST be accessible via keyboard:

- Dashboard grid: arrow keys move between cards
- Cards: `Enter`/`Space` activates
- Context menus: arrow keys navigate, `Enter` selects, `Esc` closes
- Modals: `Tab` cycles through fields, `Enter` submits

### Reduced Motion

When `prefers-reduced-motion: reduce` is set:

- Disable all `transform` animations (scale, translate)
- Disable pulse effects (token refresh pulse)
- Disable shimmer (use static grey)
- Keep opacity transitions (fade in/out) but shorten to 50ms
- Quota bar fills instantly (no interpolation)
