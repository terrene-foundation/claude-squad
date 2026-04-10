# csq v2.0 — Component Library

Every component specified with TypeScript props, visual states, and Svelte 5 skeleton code. Components use runes (`$state`, `$derived`, `$effect`, `$props`).

## Import Pattern

```typescript
// src/lib/components/index.ts
export { default as AccountCard } from "./AccountCard.svelte";
export { default as QuotaBar } from "./QuotaBar.svelte";
export { default as TokenHealth } from "./TokenHealth.svelte";
export { default as AccountSwitcher } from "./AccountSwitcher.svelte";
export { default as StatusBadge } from "./StatusBadge.svelte";
export { default as RefreshIndicator } from "./RefreshIndicator.svelte";
export { default as OAuthLoginModal } from "./OAuthLoginModal.svelte";
export { default as SettingsPanel } from "./SettingsPanel.svelte";
export { default as ToastStack } from "./ToastStack.svelte";
export { default as Button } from "./primitives/Button.svelte";
export { default as Badge } from "./primitives/Badge.svelte";
export { default as Icon } from "./primitives/Icon.svelte";
```

---

## AccountCard

The central component. Shows an account's identity, quota, and health at a glance.

### Props

```typescript
interface AccountCardProps {
  account: {
    id: number;
    email: string;
    subscriptionType: "max" | "pro" | "free" | null;
    source: "Anthropic" | "ThirdParty" | "Manual";
  };
  quota: {
    fiveHourPct: number | null;
    fiveHourResetsAt: number | null; // Unix seconds
    sevenDayPct: number | null;
    sevenDayResetsAt: number | null;
  };
  tokenStatus: "valid" | "expiring" | "expired" | "refreshing" | "login-needed";
  isActive: boolean;
  onActivate?: () => void;
  onDetails?: () => void;
}
```

### Visual States

| State        | Background                             | Border             | Accent                      |
| ------------ | -------------------------------------- | ------------------ | --------------------------- |
| Default      | `--bg-surface`                         | `--border-subtle`  | —                           |
| Hover        | `--bg-hover`                           | `--border-default` | —                           |
| Active       | `--bg-surface` + `--accent-muted` tint | `--accent`         | left edge glow              |
| Refreshing   | `--bg-surface`                         | `--border-default` | pulsing `--accent-glow`     |
| Login-needed | `--bg-surface`                         | `--status-danger`  | left edge `--status-danger` |

### Layout (ASCII wireframe)

```
┌─ 360px ────────────────────────────────────────┐
│ ●  Account #3                          ✓ Valid │  ← header row
│    user@example.com                            │
│    max · default_claude_max_20x                │  ← metadata
│                                                │
│    5h ████████████░░░░░░░░  62%  resets 2h 14m │  ← 5h bar
│    7d ██████░░░░░░░░░░░░░░  28%  resets 4d  6h │  ← 7d bar
│                                                │
│  [ Activate ]                      [ Details ▸]│  ← actions (on hover)
└────────────────────────────────────────────────┘
```

### Svelte Skeleton

```svelte
<script lang="ts">
  import QuotaBar from './QuotaBar.svelte';
  import TokenHealth from './TokenHealth.svelte';
  import Badge from './primitives/Badge.svelte';
  import Button from './primitives/Button.svelte';
  import Icon from './primitives/Icon.svelte';

  let { account, quota, tokenStatus, isActive, onActivate, onDetails } = $props<AccountCardProps>();

  let cardClass = $derived([
    'account-card',
    isActive && 'account-card--active',
    tokenStatus === 'login-needed' && 'account-card--danger',
    tokenStatus === 'refreshing' && 'account-card--refreshing',
  ].filter(Boolean).join(' '));

  const subtypeLabel = $derived(
    account.subscriptionType ? account.subscriptionType.toUpperCase() : ''
  );
</script>

<article class={cardClass} role="region" aria-label={`Account ${account.id}`}>
  <header class="card-header">
    <div class="identity">
      {#if isActive}
        <Icon name="zap" size={14} class="active-icon" aria-label="Active account" />
      {/if}
      <span class="account-num">#{account.id}</span>
      <span class="email">{account.email}</span>
    </div>
    <TokenHealth status={tokenStatus} />
  </header>

  {#if account.subscriptionType}
    <div class="metadata">
      <Badge variant="subtle">{subtypeLabel}</Badge>
    </div>
  {/if}

  <div class="quota-stack">
    <QuotaBar
      label="5h"
      percentage={quota.fiveHourPct}
      resetsAt={quota.fiveHourResetsAt}
    />
    <QuotaBar
      label="7d"
      percentage={quota.sevenDayPct}
      resetsAt={quota.sevenDayResetsAt}
    />
  </div>

  <footer class="card-actions">
    {#if !isActive}
      <Button variant="primary" size="sm" onclick={onActivate}>Activate</Button>
    {/if}
    <Button variant="ghost" size="sm" onclick={onDetails}>
      Details <Icon name="chevron-right" size={14} />
    </Button>
  </footer>
</article>

<style>
  .account-card {
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    transition:
      background var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
    position: relative;
  }

  .account-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-default);
  }

  .account-card--active {
    background: linear-gradient(
      to right,
      var(--accent-muted),
      var(--bg-surface) 20%
    );
    border-color: var(--accent);
  }

  .account-card--active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 12px;
    bottom: 12px;
    width: 3px;
    background: var(--accent);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    box-shadow: var(--shadow-glow-accent);
  }

  .account-card--danger {
    border-color: var(--status-danger);
  }

  .account-card--refreshing {
    animation: refreshPulse 1.6s var(--ease-in-out) infinite;
  }

  @keyframes refreshPulse {
    0%, 100% { box-shadow: 0 0 0 0 var(--accent-glow); }
    50% { box-shadow: 0 0 0 4px var(--accent-glow); }
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }

  .identity {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .account-num {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    font-weight: var(--weight-medium);
  }

  .email {
    font-size: var(--text-md);
    color: var(--text-primary);
    font-weight: var(--weight-medium);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .metadata {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .quota-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .card-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-3);
    opacity: 0;
    transition: opacity var(--duration-fast);
  }

  .account-card:hover .card-actions {
    opacity: 1;
  }
</style>
```

---

## QuotaBar

Dual-purpose component — renders one quota window (5h or 7d).

### Props

```typescript
interface QuotaBarProps {
  label: string; // "5h" | "7d"
  percentage: number | null; // 0-100, null = no data
  resetsAt: number | null; // Unix seconds
}
```

### Visual States

Color interpolation based on percentage:

- 0-70%: `--status-healthy` (green)
- 70-85%: `--status-warning` (amber)
- 85-100%: `--status-danger` (red)

The fill animates from 0 → target on mount, and smoothly interpolates when the percentage changes.

### Layout

```
┌──────────────────────────────────────────────┐
│ 5h ████████████░░░░░░░░  62%  resets 2h 14m │
│    │          │          │        │          │
│    └─ fill    └─ track   └─ %     └─ countdown│
└──────────────────────────────────────────────┘
```

### Svelte Skeleton

```svelte
<script lang="ts">
  let { label, percentage, resetsAt } = $props<QuotaBarProps>();

  const pct = $derived(percentage ?? 0);
  const hasData = $derived(percentage !== null);

  const barColor = $derived(
    pct >= 85 ? 'var(--status-danger)' :
    pct >= 70 ? 'var(--status-warning)' :
    'var(--status-healthy)'
  );

  let now = $state(Math.floor(Date.now() / 1000));
  $effect(() => {
    const id = setInterval(() => {
      now = Math.floor(Date.now() / 1000);
    }, 30000);  // update every 30s — fine-grained enough for minutes display
    return () => clearInterval(id);
  });

  const resetsIn = $derived(
    resetsAt && resetsAt > now ? resetsAt - now : null
  );

  function fmtTime(secs: number): string {
    if (secs < 60) return 'now';
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) {
      const h = Math.floor(secs / 3600);
      const m = Math.floor((secs % 3600) / 60);
      return m > 0 ? `${h}h ${m}m` : `${h}h`;
    }
    return `${Math.floor(secs / 86400)}d`;
  }
</script>

<div class="quota-row" role="group" aria-label={`${label} quota`}>
  <span class="label">{label}</span>
  <div class="bar-track">
    {#if hasData}
      <div
        class="bar-fill"
        style:width="{pct}%"
        style:background={barColor}
      />
    {/if}
  </div>
  <span class="percentage">
    {hasData ? `${pct.toFixed(0)}%` : '—'}
  </span>
  {#if resetsIn !== null}
    <span class="resets" title="Resets at {new Date(resetsAt * 1000).toLocaleString()}">
      {fmtTime(resetsIn)}
    </span>
  {/if}
</div>

<style>
  .quota-row {
    display: grid;
    grid-template-columns: 24px 1fr 40px 60px;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .label {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  .bar-track {
    height: 6px;
    background: var(--bg-deep);
    border-radius: var(--radius-full);
    overflow: hidden;
    box-shadow: var(--shadow-inset);
  }

  .bar-fill {
    height: 100%;
    border-radius: var(--radius-full);
    transition: width var(--duration-slow) var(--ease-out),
                background var(--duration-normal) var(--ease-out);
  }

  .percentage {
    color: var(--text-secondary);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .resets {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    text-align: right;
  }
</style>
```

---

## TokenHealth

Small badge showing token status with icon + short label.

### Props

```typescript
interface TokenHealthProps {
  status: "valid" | "expiring" | "expired" | "refreshing" | "login-needed";
}
```

### States

| Status       | Icon                    | Label        | Color              |
| ------------ | ----------------------- | ------------ | ------------------ |
| valid        | `check-circle-2`        | Valid        | `--status-healthy` |
| expiring     | `clock`                 | Expiring     | `--status-warning` |
| expired      | `alert-triangle`        | Expired      | `--status-warning` |
| refreshing   | `refresh-cw` (spinning) | Refreshing   | `--accent`         |
| login-needed | `x-circle`              | LOGIN-NEEDED | `--status-danger`  |

### Svelte Skeleton

```svelte
<script lang="ts">
  import Icon from './primitives/Icon.svelte';

  let { status } = $props<TokenHealthProps>();

  const config = $derived({
    valid: { icon: 'check-circle-2', label: 'Valid', color: 'var(--status-healthy)' },
    expiring: { icon: 'clock', label: 'Expiring', color: 'var(--status-warning)' },
    expired: { icon: 'alert-triangle', label: 'Expired', color: 'var(--status-warning)' },
    refreshing: { icon: 'refresh-cw', label: 'Refreshing', color: 'var(--accent)' },
    'login-needed': { icon: 'x-circle', label: 'LOGIN-NEEDED', color: 'var(--status-danger)' },
  }[status]);
</script>

<span class="token-health" style:color={config.color}>
  <Icon
    name={config.icon}
    size={12}
    class={status === 'refreshing' ? 'spin' : ''}
  />
  <span class="label">{config.label}</span>
</span>

<style>
  .token-health {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wide);
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
```

---

## Primitives: Button

```typescript
interface ButtonProps {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  loading?: boolean;
  onclick?: (e: MouseEvent) => void;
  children: Snippet;
}
```

Styles follow the design system — primary uses `--accent`, ghost uses transparent with hover fill. 28/32/36px heights for sm/md/lg.

---

## Primitives: Badge

```typescript
interface BadgeProps {
  variant?: "default" | "subtle" | "success" | "warning" | "danger";
  size?: "sm" | "md";
}
```

Small pill-shaped label. Used for subscription tier, account source, status indicators.

---

## Primitives: Icon

Svelte wrapper around Lucide icons with consistent sizing.

```typescript
interface IconProps {
  name: string; // Lucide icon name
  size?: number; // px, default 16
  class?: string;
  "aria-label"?: string;
}
```

Uses `lucide-svelte` package — tree-shakes unused icons automatically.

---

## ToastStack

Non-blocking notifications. Slide in from top-right, auto-dismiss after 4s.

### Props

```typescript
interface Toast {
  id: string;
  variant: "info" | "success" | "warning" | "error";
  title: string;
  description?: string;
  duration?: number; // ms, default 4000, 0 = sticky
  action?: { label: string; onClick: () => void };
}

interface ToastStackProps {
  toasts: Toast[];
  onDismiss: (id: string) => void;
}
```

### Position

Fixed `top: 16px; right: 16px`. Stack vertically with 8px gap, newest on top. Max 5 visible; older ones auto-dismiss.

### Motion

Entry: `translateX(100%)` → `translateX(0)` with spring overshoot (`--ease-spring`, 280ms).
Exit: `opacity 1 → 0` + `translateX(20%)`, 150ms.

---

## SystemTrayMenu

Native menu — built via Tauri `tray` API, not Svelte. Specified here for reference.

### Structure

```
[icon state changes by active account]

── Active: #3 user@example.com ────────
   5h 62%   7d 28%
   Token: ✓ Valid

── Switch Account ─────────────────────
   ○ #1 other@example.com   5h 14%
   ● #3 user@example.com    5h 62%  (active)
   ○ #5 team@example.com    5h 91% ⚠
   ○ #7 admin@example.com   5h 100% ⛔

── ────────────────────────────────────
   Open Dashboard         ⌘ O
   Refresh All            ⌘ R
   Preferences...         ⌘ ,
── ────────────────────────────────────
   Quit csq               ⌘ Q
```

Menu rebuilds on every account state change (use Tauri `TrayIcon::set_menu`).

---

## OAuthLoginModal

Multi-step flow for adding a new account via OAuth PKCE.

### Props

```typescript
interface OAuthLoginModalProps {
  open: boolean;
  onClose: () => void;
  onComplete: (account: number) => void;
}
```

### Steps

1. **Idle** — "Add new Claude account" with big button "Continue in Browser"
2. **Browser opened** — spinner + "Waiting for you to authorize in your browser..." with cancel button
3. **Capturing credentials** — progress bar + "Saving credentials..."
4. **Done** — checkmark + account number, auto-close after 1.5s

### Error States

- Timeout after 5 minutes
- User cancelled in browser
- Network error
- Credentials capture failed

Each error shows an actionable recovery option (retry, open browser again, report issue).

---

## SettingsPanel

Tabbed panel for preferences.

### Tabs

1. **General** — refresh interval, auto-rotation toggle, launch at login, start minimized
2. **Accounts** — list of discovered accounts, reorder, remove, add manual
3. **Providers** — Z.AI, MiniMax, other 3P providers (set API key)
4. **Appearance** — theme (dark/light/system), accent color, compact mode
5. **Advanced** — config dir path, export logs, reset state

Each tab is a separate Svelte component loaded on demand. Settings persist via Tauri `store` plugin.

---

## Loading States

All components specify a loading skeleton with the same shape as the loaded content:

```svelte
{#if loading}
  <div class="skeleton" style:height="64px" />
{:else}
  <!-- actual content -->
{/if}

<style>
  .skeleton {
    background: linear-gradient(
      90deg,
      var(--bg-surface) 0%,
      var(--bg-hover) 50%,
      var(--bg-surface) 100%
    );
    background-size: 200% 100%;
    border-radius: var(--radius-lg);
    animation: shimmer 1.6s var(--ease-in-out) infinite;
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
```

---

## Empty States

Used when no accounts are configured, no search results, or no history. Show:

1. Large icon (32px)
2. Headline ("No accounts yet")
3. One-line description
4. Primary action button

```
┌─────────────────────────────┐
│                             │
│         ╭─ user ─╮          │
│         │   +    │          │
│         ╰────────╯          │
│                             │
│     No accounts yet         │
│                             │
│  Add your first Claude      │
│  account to start tracking  │
│  quota and rotating tokens. │
│                             │
│    [ + Add Account ]        │
│                             │
└─────────────────────────────┘
```

---

## Accessibility Requirements

Every interactive component MUST:

1. Be reachable by `Tab` key
2. Have a visible focus ring (the `:focus-visible` default style)
3. Have an `aria-label` if icon-only
4. Support keyboard activation (`Enter`/`Space` on buttons, arrows on menus)
5. Announce state changes via `aria-live="polite"` regions for toasts
6. Respect `prefers-reduced-motion` — disable all animations
