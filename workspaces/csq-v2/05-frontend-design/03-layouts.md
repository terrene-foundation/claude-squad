# csq v2.0 — Page Layouts

All layouts shown as ASCII wireframes at the default 720×520 window size.

## App Shell

Every window uses the same shell:

```
┌─ 720×520 ───────────────────────────────────────────────────┐
│ ●○○  csq                                           ⚙  ⊟  ×  │  ← native title bar (28px)
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [ Page content ]                                           │  ← scrollable content area
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ ● Active: #3 user@example.com  ·  5h 62%  ·  7d 28%  ·  ✓  │  ← status bar (32px)
└─────────────────────────────────────────────────────────────┘
```

**Title bar**: native (traffic lights on macOS, min/max/close on Windows/Linux). No custom chrome.

**Status bar**: always visible at bottom, shows active account + overall health. Click to open account switcher.

---

## Dashboard (Main Window)

The default view. Grid of account cards with quick stats and actions.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ●○○  csq                                           ⚙  ⊟  ×   │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   Dashboard                             [ + Add Account ]    │  ← header
│   7 accounts  ·  2 active  ·  auto-rotate on                 │  ← summary
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────┐         │
│  │ ● #1  [user@ex.com]  │  │    #2  [dev@ex.com]  │         │
│  │       max            │  │        max           │         │
│  │ 5h ████░░░ 14%  2h4m │  │ 5h ██░░░░░ 8%   4h1m │         │
│  │ 7d ██░░░░░  5%  3d0h │  │ 7d █░░░░░░ 2%   5d2h │         │
│  │                 ✓ ok │  │                 ✓ ok │         │
│  └──────────────────────┘  └──────────────────────┘         │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────┐         │
│  │    #3  [team@ex.com] │  │    #5  [ops@ex.com]  │         │
│  │        max           │  │        max           │         │
│  │ 5h ████████ 62% 2h14m│  │ 5h █████████ 91% 1h5m│         │
│  │ 7d ███░░░░ 28%  4d6h │  │ 7d ██████░ 52%  3d8h │         │
│  │                 ✓ ok │  │                 ⚠ wrn│         │
│  └──────────────────────┘  └──────────────────────┘         │
│                                                              │
│  ┌──────────────────────┐                                    │
│  │    #7  [old@ex.com]  │                                    │
│  │        pro           │                                    │
│  │ 5h ██████████ 100%   │                                    │
│  │ 7d ██████░ 67%  5d0h │                                    │
│  │        ⛔ LOGIN-NEED │                                    │
│  └──────────────────────┘                                    │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ ● Active: #3 team@ex.com  ·  5h 62%  ·  7d 28%  ·  ✓        │
└──────────────────────────────────────────────────────────────┘
```

### Grid

CSS Grid with auto-fit columns:

```css
.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--space-4);
  padding: var(--space-4);
}
```

- At 720px window: 2 columns
- At 960px: 2 columns (cards get wider)
- At 1200px: 3 columns

### Header

```
Dashboard                             [ + Add Account ]
7 accounts  ·  2 active  ·  auto-rotate on
```

- Page title (`--text-xl`, `--weight-semibold`)
- Primary action button on the right
- Summary line below (`--text-sm`, `--text-tertiary`)

### Sorting & Filtering

Keyboard shortcut `Cmd/Ctrl+F` opens a search bar above the grid. Filter by:

- Account number
- Email
- Status (valid/warning/login-needed)
- Source (Anthropic/3P/Manual)

Default sort: active account first, then by 5h usage ascending (lowest first).

### Empty State

No accounts configured:

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                       ╭─ user ─╮                             │
│                       │   +    │                             │
│                       ╰────────╯                             │
│                                                              │
│                   No accounts yet                            │
│                                                              │
│           Add your first Claude account to start             │
│            tracking quota and rotating tokens.               │
│                                                              │
│                 [ + Add Account ]                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Account Detail

Expanded view of a single account. Shown when user clicks "Details ▸" on a card.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ◂ Back       Account #3                              ⚙  ⊟  × │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│   ● #3  team@example.com                    [ Activate ]     │
│   max · default_claude_max_20x                               │
│                                                              │
│   ──────────────────────────────────────────────────────    │
│                                                              │
│   Usage                                                      │
│                                                              │
│   5-hour window                           62%  resets 2h 14m │
│   ████████████████████████░░░░░░░░░░░░░░░░░░░░░░░            │
│   Updated 34s ago                                            │
│                                                              │
│   7-day window                             28%  resets 4d 6h │
│   ███████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░           │
│   Updated 34s ago                                            │
│                                                              │
│   ──────────────────────────────────────────────────────    │
│                                                              │
│   Token                                                      │
│                                                              │
│   Status        ✓ Valid                                      │
│   Expires       in 2h 14m (2026-04-10 18:24:41 UTC)          │
│   Scopes        inference, profile, mcp_servers              │
│   Last refresh  2h 46m ago                                   │
│                                                              │
│   [ Refresh Now ]     [ Re-login ]                           │
│                                                              │
│   ──────────────────────────────────────────────────────    │
│                                                              │
│   Sessions (2 active)                                        │
│                                                              │
│   ● config-3   PID 12345  started 3h ago                     │
│   ● config-31  PID 12890  started 1h ago                     │
│                                                              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ ● Active: #3 team@ex.com  ·  5h 62%  ·  7d 28%  ·  ✓        │
└──────────────────────────────────────────────────────────────┘
```

### Sections

1. **Header** — identity, primary action, back button
2. **Usage** — full-width quota bars with timestamps
3. **Token** — status, expiry, scopes, refresh actions
4. **Sessions** — live list of CC processes using this account

Sections separate with horizontal rules (`--border-subtle`).

---

## Settings

Tabbed layout. Sidebar navigation on the left, content on the right.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ◂ Back       Settings                                ⊟  ×    │
├───────────────┬──────────────────────────────────────────────┤
│               │                                              │
│  ▸ General    │  General                                     │
│    Accounts   │                                              │
│    Providers  │  Refresh interval                            │
│    Appearance │  Check tokens every  [ 5 min ▾ ]             │
│    Advanced   │                                              │
│               │  Auto-rotation                               │
│               │  [ ● ] Automatically swap when quota low     │
│               │       Threshold: [ 95% ▾ ]                   │
│               │                                              │
│               │  Startup                                     │
│               │  [ ● ] Launch at login                       │
│               │  [ ○ ] Start minimized to tray               │
│               │                                              │
│               │  Dashboard                                   │
│               │  [ ● ] Show status bar                       │
│               │  [ ○ ] Compact account cards                 │
│               │                                              │
│               │                                              │
├───────────────┴──────────────────────────────────────────────┤
│ ● Active: #3 team@ex.com  ·  5h 62%  ·  7d 28%  ·  ✓        │
└──────────────────────────────────────────────────────────────┘
```

### Sidebar

Fixed 160px width, list of tab labels. Active tab has `--accent-muted` background and `--accent` left border.

### Content

Scrollable. Sections separated by `var(--space-6)`. Each section has:

- Small uppercase label (`--text-xs`, `--text-tertiary`, `--tracking-wide`)
- Form controls with labels
- Help text below inputs where needed

---

## OAuth Login Modal

Overlays the dashboard. Darkens background with 60% opacity.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│ ░░░┌────────────────────────────────────────────────┐░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░│            ╭──── key ────╮                     │░░░░░ │
│ ░░░│            │             │                     │░░░░░ │
│ ░░░│            ╰─────────────╯                     │░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░│         Add New Account                        │░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░│   Sign in with your Claude account. A new      │░░░░░ │
│ ░░░│   browser tab will open for you to authorize.  │░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░│      [ Cancel ]    [ Continue in Browser ]     │░░░░░ │
│ ░░░│                                                │░░░░░ │
│ ░░░└────────────────────────────────────────────────┘░░░░░ │
│ ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
└──────────────────────────────────────────────────────────────┘
```

Modal width: 400px. Height: auto (~280px for idle state).

### Flow States

**Idle**: Icon + heading + description + two buttons (Cancel, Continue in Browser)

**Waiting**: Spinner + "Waiting for authorization in browser..." + progress dots + Cancel button

**Processing**: Progress bar + "Saving credentials..." + account number

**Done**: Checkmark icon + "Account #4 added!" + auto-dismiss after 1.5s

**Error**: Warning icon + error message + Retry + Cancel buttons

---

## First Run / Onboarding

Shown when no accounts exist AND no previous csq state found.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                                                              │
│                        ╭──── key ────╮                       │
│                        │             │                       │
│                        ╰─────────────╯                       │
│                                                              │
│                       Welcome to csq                         │
│                                                              │
│       Manage multiple Claude Code accounts with ease.        │
│         Automatic token refresh. Smart rotation.             │
│                 All your quota in one place.                 │
│                                                              │
│                                                              │
│                   [ Get Started ]                            │
│                                                              │
│                                                              │
│         Already have accounts? [ Import existing ]           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Two paths:

1. **Get Started** → OAuth login flow
2. **Import existing** → scan `~/.claude/accounts` for existing configs

---

## System Tray Popover (macOS-specific)

When the user clicks the menu bar icon on macOS, a popover appears instead of a menu. 360×480 fixed, no window chrome.

```
┌─────── 360×480 ────────────┐
│                            │
│  csq                    ⚙  │  ← mini header (40px)
│                            │
│ ─────────────────────────  │
│                            │
│  ● #3 team@ex.com          │  ← active account card (full)
│     max                    │
│     5h ████░░ 62%   2h 14m │
│     7d ██░░░░ 28%   4d  6h │
│     ✓ Token valid          │
│                            │
│  [ Open Dashboard ]        │
│                            │
│ ─────────────────────────  │
│                            │
│  SWITCH ACCOUNT            │  ← compact list
│                            │
│  ○ #1  other@ex     14% →  │
│  ○ #2  dev@ex        8% →  │
│  ○ #5  ops@ex       91% →  │
│  ○ #7  old@ex      100% →  │
│                            │
│ ─────────────────────────  │
│                            │
│  Refresh All    Quit       │  ← footer (40px)
│                            │
└────────────────────────────┘
```

On Linux/Windows, use a traditional dropdown menu instead of a popover (see `05-system-tray.md`).

---

## Responsive Behavior

csq is a desktop app with a fixed window range (480-1200px). No mobile. But content should adapt gracefully:

| Width      | Layout                                     |
| ---------- | ------------------------------------------ |
| 480-640px  | 1 column, compact cards, hide metadata row |
| 640-960px  | 2 columns, full cards                      |
| 960-1200px | 3 columns, full cards                      |

At minimum width (480px), some elements collapse:

- Status bar hides the email, only shows `#N` + quotas
- Account cards hide `metadata` row
- Settings sidebar collapses to a hamburger

## Status Bar Detail

The always-visible status bar at the window bottom:

```
┌──────────────────────────────────────────────────────────────┐
│ ● Active: #3 team@example.com  ·  5h 62%  ·  7d 28%  ·  ✓   │
│ │         │                      │          │          │    │
│ │         └─ identity            └─ quotas  │          │    │
│ └─ pulse dot                                 └─ health  │    │
│                                                         └─ click to switch
└──────────────────────────────────────────────────────────────┘
```

- **Pulse dot**: small (6px) circle in `--status-healthy` that pulses every 4s to show the app is live
- **Identity**: clickable, opens account detail
- **Quotas**: tabular numerals, updates every 30s
- **Health**: ✓ / ⚠ / ⛔ icon based on worst status across windows
- **Click target**: entire bar is a click handler that opens the account switcher
