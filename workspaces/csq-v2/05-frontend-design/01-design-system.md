# csq v2.0 — Design System

Modern, glanceable desktop tool aesthetic. Dark-first. Native feel across macOS/Linux/Windows. Information-dense but not cluttered.

## Philosophy

csq is a **developer utility**, not a consumer app. Users are watching it from the corner of their eye while coding. Design for:

1. **Glanceability** — most important info (active account, 5h %, token health) visible in <200ms
2. **Quiet by default** — the app stays out of the way; only speaks up on state changes
3. **Native feel** — system fonts, native dialogs, platform-appropriate tray behavior
4. **Information density** — devs want data, not whitespace; pack tight but readable
5. **Precision** — exact percentages, timestamps, token counts; no rounding for aesthetics

## Visual Language

### Color Palette — Dark (default)

```css
/* Neutrals — deep slate with subtle blue undertone */
--bg-deep: #0a0e14; /* App background, window chrome */
--bg-surface: #10151b; /* Cards, panels — slightly lifted */
--bg-elevated: #161c25; /* Modals, dropdowns, popovers */
--bg-hover: #1c2330; /* Interactive hover state */
--bg-active: #232b3a; /* Interactive pressed state */

/* Borders — subtle, never harsh */
--border-subtle: rgba(255, 255, 255, 0.06);
--border-default: rgba(255, 255, 255, 0.1);
--border-strong: rgba(255, 255, 255, 0.16);

/* Text */
--text-primary: #e8ecf1; /* Headings, primary content */
--text-secondary: #a8b2c1; /* Body text, labels */
--text-tertiary: #6a7588; /* Captions, metadata */
--text-disabled: #404a5c; /* Disabled state */

/* Signature accent — a warm cyan, not corporate blue */
--accent: #06d4c4; /* Primary brand accent */
--accent-hover: #1ae4d4;
--accent-muted: rgba(6, 212, 196, 0.12); /* Tint backgrounds */
--accent-glow: rgba(6, 212, 196, 0.35); /* Glows, halos */

/* Semantic — health indicators */
--status-healthy: #4ade80; /* <80% quota, token valid */
--status-warning: #fbbf24; /* 80-99% quota, expiring soon */
--status-danger: #f87171; /* 100% quota, LOGIN-NEEDED, errors */
--status-neutral: #94a3b8; /* Idle, no data */

/* Usage bar gradients — smooth transitions */
--quota-low: #4ade80;
--quota-mid: #fbbf24;
--quota-high: #fb923c;
--quota-full: #f87171;
```

### Color Palette — Light (secondary)

```css
--bg-deep: #f7f9fc;
--bg-surface: #ffffff;
--bg-elevated: #ffffff;
--bg-hover: #eef1f6;
--bg-active: #e2e7ef;

--border-subtle: rgba(0, 0, 0, 0.06);
--border-default: rgba(0, 0, 0, 0.1);
--border-strong: rgba(0, 0, 0, 0.18);

--text-primary: #0a0e14;
--text-secondary: #3a4456;
--text-tertiary: #6a7588;
--text-disabled: #a8b2c1;

--accent: #00a99a; /* Slightly darker for contrast */
--accent-hover: #008a7d;
--accent-muted: rgba(0, 169, 154, 0.1);
--accent-glow: rgba(0, 169, 154, 0.25);

--status-healthy: #16a34a;
--status-warning: #d97706;
--status-danger: #dc2626;
--status-neutral: #64748b;
```

### Glassmorphism (macOS)

```css
--glass-bg: rgba(16, 21, 27, 0.72);
--glass-blur: 24px;
--glass-border: rgba(255, 255, 255, 0.08);
--glass-shadow:
  0 1px 0 rgba(255, 255, 255, 0.04) inset, 0 20px 40px -12px rgba(0, 0, 0, 0.6);
```

Use on the main window background and tray popover. Falls back to `--bg-surface` on Linux/Windows without backdrop-filter support.

## Typography

### Font Stack

```css
--font-sans:
  -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", "Inter",
  "Helvetica Neue", sans-serif;
--font-mono:
  "SF Mono", "Roboto Mono", "JetBrains Mono", ui-monospace, Menlo, Consolas,
  monospace;
```

No web fonts. No Google Fonts. System stack only — instant render, zero network, native feel.

### Type Scale

Compact and readable. 12px base because this is a utility — devs are already used to terminal font sizes.

```css
--text-xs: 10px; /* Micro labels, badges */
--text-sm: 11px; /* Captions, metadata, counters */
--text-base: 12px; /* Body, list items */
--text-md: 13px; /* Card titles, emphasized body */
--text-lg: 15px; /* Section headers */
--text-xl: 18px; /* Page titles */
--text-2xl: 22px; /* Hero numbers (quota %, token counts) */

--leading-tight: 1.2;
--leading-normal: 1.4;
--leading-relaxed: 1.6;

--tracking-tight: -0.01em; /* Large numbers */
--tracking-normal: 0;
--tracking-wide: 0.02em; /* Uppercase labels */
```

### Weights

```css
--weight-normal: 400;
--weight-medium: 500;
--weight-semibold: 600;
--weight-bold: 700;
```

Reserve bold for section titles and the active account indicator only. Medium for labels and emphasis. Normal for body.

## Spacing

4px base unit. Never use arbitrary pixel values — pick from the scale.

```css
--space-0: 0;
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 20px;
--space-6: 24px;
--space-8: 32px;
--space-10: 40px;
--space-12: 48px;
```

Layout rhythm: cards pad `--space-3` to `--space-4`, sections separate with `--space-6`, page margins `--space-4`.

## Radii

Generous but not cartoonish.

```css
--radius-sm: 4px; /* Badges, inline tags */
--radius-md: 6px; /* Buttons, inputs */
--radius-lg: 8px; /* Cards, panels */
--radius-xl: 12px; /* Modal windows */
--radius-full: 9999px; /* Pills, avatars */
```

## Shadows

Layered — inner highlight on top, outer drop on bottom. No harsh black shadows.

```css
/* Elevation — subtle outer shadows */
--shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.08), 0 0 0 1px rgba(255, 255, 255, 0.03);
--shadow-md:
  0 4px 8px -2px rgba(0, 0, 0, 0.12), 0 0 0 1px rgba(255, 255, 255, 0.04);
--shadow-lg:
  0 12px 24px -4px rgba(0, 0, 0, 0.24), 0 0 0 1px rgba(255, 255, 255, 0.05);
--shadow-xl:
  0 24px 48px -8px rgba(0, 0, 0, 0.32), 0 0 0 1px rgba(255, 255, 255, 0.06);

/* Inset — for pressed panels or quota bar tracks */
--shadow-inset: inset 0 1px 2px rgba(0, 0, 0, 0.2);

/* Glow — for active states, token refresh pulse */
--shadow-glow-accent: 0 0 20px var(--accent-glow);
--shadow-glow-danger: 0 0 16px rgba(248, 113, 113, 0.3);
```

## Motion

Motion reinforces the mental model, never decorates. Keep durations short (150-400ms) and easings natural.

```css
--ease-out: cubic-bezier(0.16, 1, 0.3, 1); /* Decelerate — entry */
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1); /* Smooth — transitions */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* Overshoot — toasts */

--duration-fast: 150ms;
--duration-normal: 250ms;
--duration-slow: 400ms;
```

Motion principles:

- **Entry**: opacity 0 → 1 + translateY(4px → 0), `--ease-out`, `--duration-normal`
- **Exit**: opacity 1 → 0 (no translate), `--duration-fast`
- **State change**: background/color interpolates over `--duration-fast`
- **Quota bar fill**: width interpolates, `--ease-out`, `--duration-slow`
- **Refresh pulse**: `box-shadow` pulses for 800ms on token refresh
- **No bouncing** except toasts (subtle spring overshoot)
- **Respect `prefers-reduced-motion`** — disable all transforms and pulses

## Iconography

**Lucide Icons** (MIT license, tree-shakeable, 1000+ icons). Stroke-based, consistent 24px viewBox, `stroke-width: 1.5` for our small sizes.

Usage:

- **16px** — inline with text (list items, buttons)
- **20px** — card headers, panel titles
- **24px** — tray icon, empty state
- **32px** — hero illustrations

Key icons:

- `user-circle` — account avatar
- `key` — token/credential
- `refresh-cw` — refresh action, spinning during refresh
- `check-circle-2` — healthy/valid
- `alert-triangle` — warning (near limit)
- `x-circle` — error (LOGIN-NEEDED)
- `zap` — active account
- `clock` — reset countdown
- `chevron-down` — dropdown
- `plus` — add account
- `settings` — preferences
- `minimize-2` — minimize to tray

## Window Constraints

```typescript
{
  width: { min: 480, default: 720, max: 1200 },
  height: { min: 360, default: 520, max: 800 },
  decorations: 'native',     // use platform title bar
  transparent: true,         // for glassmorphism (macOS only)
  alwaysOnTop: false,
  resizable: true,
  fullscreen: false,
}
```

Tray popover (macOS/Windows): fixed 360×480, no resize, no title bar, dismisses on click outside.

## Accessibility Baseline

- **WCAG AA contrast** — all text meets 4.5:1 minimum (7:1 for body where possible)
- **Focus rings** — visible 2px `--accent` ring with 2px offset on keyboard focus
- **Keyboard navigation** — Tab through all interactive elements in logical order
- **Screen reader** — `aria-label` on icon-only buttons, `aria-live` regions for toasts
- **No animations on `prefers-reduced-motion`** — respect the user's OS setting
- **Clickable targets** — minimum 28×28px (relaxed from 44px because this is a desktop utility)

## Platform Adaptations

| Feature        | macOS                           | Windows                   | Linux                           |
| -------------- | ------------------------------- | ------------------------- | ------------------------------- |
| Window chrome  | Native traffic lights           | Native close/min/max      | GTK headerbar                   |
| Tray icon      | Menu bar extra (template image) | System tray (multi-state) | AppIndicator (multi-state)      |
| Glassmorphism  | Full `backdrop-filter`          | `--bg-surface` fallback   | `--bg-surface` fallback         |
| Native dialogs | macOS standard                  | Windows standard          | GTK/Qt standard                 |
| Shortcuts      | ⌘ Cmd-based                     | Ctrl-based                | Ctrl-based                      |
| Font           | SF Pro                          | Segoe UI                  | System default (Inter fallback) |

## Global CSS Reset

```css
:root {
  color-scheme: dark;
  font-family: var(--font-sans);
  font-size: var(--text-base);
  line-height: var(--leading-normal);
  color: var(--text-primary);
  background: var(--bg-deep);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  font-feature-settings: "tnum" 1; /* Tabular numerals for percentages */
}

*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

button,
input,
select,
textarea {
  font: inherit;
  color: inherit;
}

button {
  background: none;
  border: none;
  cursor: pointer;
}

:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```
