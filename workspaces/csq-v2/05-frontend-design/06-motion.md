# csq v2.0 — Motion Design

Motion in csq is functional, not decorative. Every animation communicates state or guides attention. Nothing wiggles, nothing bounces for fun.

## Principles

1. **Fast** — Duration 150-400ms. Never longer. Developers interact fast and get annoyed by slow transitions.
2. **Purposeful** — Each animation answers: "What changed?" or "Where did it go?"
3. **Natural** — Ease-out for entries, ease-in-out for transitions, spring only for toasts
4. **Reduce-motion aware** — Disabled entirely if OS flag is set
5. **No parallax, no blur effects** — Keep the app performant on older hardware

## Timing Tokens

```css
--duration-instant: 50ms; /* State toggle — focus, hover */
--duration-fast: 150ms; /* Color change, opacity fade */
--duration-normal: 250ms; /* Entry/exit transitions */
--duration-slow: 400ms; /* Quota fills, larger content */
--duration-slower: 600ms; /* Scale transitions on modals */
```

## Easing Curves

```css
--ease-linear: linear;
--ease-out: cubic-bezier(
  0.16,
  1,
  0.3,
  1
); /* Decelerate — default for entries */
--ease-in: cubic-bezier(0.4, 0, 1, 1); /* Accelerate — exits */
--ease-in-out: cubic-bezier(0.4, 0, 0.2, 1); /* Smooth — state transitions */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* Overshoot — toasts */
```

**When to use each**:

- `ease-out`: elements appearing (entry), content sliding in, quota bars filling
- `ease-in`: elements disappearing (exit), content sliding out
- `ease-in-out`: state changes where motion should feel balanced (accordion, tab switch)
- `ease-spring`: attention-grabbing notifications (toast in)

## Component Animations

### AccountCard — Entry

On first render (dashboard load or account added):

```css
@keyframes cardEnter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.account-card {
  animation: cardEnter var(--duration-normal) var(--ease-out) backwards;
}

/* Stagger — each card delays by 40ms */
.account-card:nth-child(1) {
  animation-delay: 0ms;
}
.account-card:nth-child(2) {
  animation-delay: 40ms;
}
.account-card:nth-child(3) {
  animation-delay: 80ms;
}
.account-card:nth-child(4) {
  animation-delay: 120ms;
}
/* ... */
```

Max stagger is 200ms total (5 cards × 40ms). Beyond that, remaining cards appear instantly to avoid long wait.

### AccountCard — Refreshing Pulse

When a token is being refreshed, the card border pulses:

```css
@keyframes refreshPulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 var(--accent-glow);
  }
  50% {
    box-shadow: 0 0 0 4px var(--accent-glow);
  }
}

.account-card--refreshing {
  animation: refreshPulse 1.6s var(--ease-in-out) infinite;
}
```

The pulse has a 1.6s period — slow enough to feel calm, fast enough to show activity.

### AccountCard — Active Switch

When the active account changes, the previously-active card's glow fades and the new card's glow grows:

```svelte
<script>
  let { isActive } = $props();
</script>

<article
  class="account-card"
  class:account-card--active={isActive}
>
  <!-- ... -->
</article>

<style>
  .account-card::before {
    content: '';
    position: absolute;
    left: 0;
    top: 12px;
    bottom: 12px;
    width: 0;
    background: var(--accent);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    transition:
      width var(--duration-normal) var(--ease-out),
      box-shadow var(--duration-normal) var(--ease-out);
  }

  .account-card--active::before {
    width: 3px;
    box-shadow: var(--shadow-glow-accent);
  }
</style>
```

Cross-fade duration 250ms — smooth enough to track the change, fast enough to feel responsive.

### QuotaBar — Fill Animation

When quota data arrives or updates, the bar fills from 0 to the target percentage:

```css
.bar-fill {
  width: 0%;
  transition:
    width var(--duration-slow) var(--ease-out),
    background var(--duration-normal) var(--ease-out);
}
```

On mount, use CSS transition with `requestAnimationFrame` trigger:

```svelte
<script>
  let { percentage } = $props();
  let animatedPct = $state(0);

  $effect(() => {
    // Next frame — allows initial 0% render
    requestAnimationFrame(() => {
      animatedPct = percentage ?? 0;
    });
  });
</script>

<div class="bar-fill" style:width="{animatedPct}%" />
```

Result: On first render the bar animates from 0% → actual %. On updates, it smoothly transitions to the new value.

**Color transitions**: When percentage crosses a threshold (70%, 85%), the color changes smoothly over 250ms — avoids jarring jumps.

### Modal — Open/Close

Modals scale up and fade in from the center:

```css
@keyframes modalOpen {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes modalClose {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.97);
  }
}

.modal {
  animation: modalOpen var(--duration-normal) var(--ease-out);
}

.modal--closing {
  animation: modalClose var(--duration-fast) var(--ease-in);
}

/* Backdrop */
.modal-backdrop {
  background: rgba(0, 0, 0, 0);
  transition: background var(--duration-normal) var(--ease-out);
}

.modal-backdrop--open {
  background: rgba(0, 0, 0, 0.6);
}
```

### Toast — Slide In

Toasts use spring easing for subtle overshoot:

```css
@keyframes toastEnter {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  60% {
    opacity: 1;
    transform: translateX(-4px); /* tiny overshoot */
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.toast {
  animation: toastEnter 280ms var(--ease-spring);
}
```

Exit is faster and simpler:

```css
@keyframes toastExit {
  to {
    opacity: 0;
    transform: translateX(20%);
  }
}

.toast--dismissing {
  animation: toastExit var(--duration-fast) var(--ease-in) forwards;
}
```

### Page Transitions — Dashboard ↔ Detail

When user clicks "Details" to go from dashboard to account detail:

1. Dashboard fades out (100ms)
2. Detail fades in + slides up 8px (200ms)

Total perceived: 250ms including slight overlap.

```svelte
<script>
  import { fade, fly } from 'svelte/transition';
</script>

{#if view === 'dashboard'}
  <div
    in:fade={{ duration: 200 }}
    out:fade={{ duration: 100 }}
  >
    <Dashboard />
  </div>
{:else if view === 'detail'}
  <div
    in:fly={{ y: 8, duration: 200, delay: 50 }}
    out:fade={{ duration: 100 }}
  >
    <AccountDetail />
  </div>
{/if}
```

### Skeleton Shimmer

Loading state shimmer is pure CSS (no JS):

```css
@keyframes shimmer {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}

.skeleton {
  background: linear-gradient(
    90deg,
    var(--bg-surface) 0%,
    var(--bg-hover) 50%,
    var(--bg-surface) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.6s var(--ease-in-out) infinite;
}
```

1.6s period matches the refresh pulse — consistent rhythm across the app.

### Tray Popover (macOS)

When the tray icon is clicked, the popover scales up from the icon position:

```css
@keyframes popoverOpen {
  from {
    opacity: 0;
    transform: scale(0.92);
    transform-origin: top right; /* anchor to tray icon */
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.tray-popover {
  animation: popoverOpen var(--duration-normal) var(--ease-out);
}
```

### Active Pulse Dot (Status Bar)

The status bar has a small pulsing dot to show the app is alive:

```css
@keyframes pulseDot {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.6;
    transform: scale(0.85);
  }
}

.pulse-dot {
  width: 6px;
  height: 6px;
  background: var(--status-healthy);
  border-radius: var(--radius-full);
  animation: pulseDot 4s var(--ease-in-out) infinite;
}
```

4-second period is slow enough to not distract, visible enough to feel alive.

### Icon Rotations

Spinning icons for refresh/loading states:

```css
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.icon-spin {
  animation: spin 1s linear infinite;
}
```

Use linear easing (not ease-in-out) — rotations should be constant velocity.

### Drag Ghost

When dragging an account card:

```css
.dragging {
  opacity: 0.5;
  transform: scale(1.02);
  box-shadow: var(--shadow-xl);
  transition:
    transform var(--duration-fast) var(--ease-out),
    box-shadow var(--duration-fast) var(--ease-out);
  z-index: 100;
}

.drop-target {
  transition: transform var(--duration-normal) var(--ease-out);
}
```

Cards shifting to make room use `--duration-normal` for smooth reflow.

## Performance Guardrails

### GPU-Accelerated Properties Only

Animate only `transform`, `opacity`, and `filter`. Never animate:

- `width` / `height` (causes reflow)
- `top` / `left` (use `transform: translate` instead)
- `background-position` (exception: shimmer, low-frequency)
- `box-shadow` (exception: pulse effects, short duration)

### Will-Change

Use `will-change` sparingly, only on elements that animate frequently:

```css
.account-card--refreshing {
  will-change: box-shadow;
}

.bar-fill {
  will-change: width; /* Yes, we violate the rule — but transitions only */
}
```

Remove `will-change` when the animation ends:

```svelte
<script>
  let isRefreshing = $state(false);
</script>

<div
  class="account-card"
  class:account-card--refreshing={isRefreshing}
  style:will-change={isRefreshing ? 'box-shadow' : 'auto'}
>
```

### Reduce Motion — Complete Override

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }

  .pulse-dot,
  .account-card--refreshing,
  .skeleton {
    animation: none;
  }

  /* Replace with static indicators */
  .pulse-dot {
    opacity: 1;
  }

  .account-card--refreshing {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .skeleton {
    background: var(--bg-hover);
  }
}
```

Static alternatives preserve meaning without motion:

- Pulsing dot → solid dot
- Pulsing border → solid outline
- Shimmer → static grey
- Slide-in toasts → instant appear (still accessible via `aria-live`)

## Testing Motion

1. **60fps baseline** — Every animation must hit 60fps on a 2018 MacBook. Profile with Chrome DevTools Performance panel.
2. **Reduce-motion** — Toggle macOS "Reduce motion" system preference; verify animations are suppressed.
3. **Slow device** — Run on a Raspberry Pi 4 (Linux target) — animations should still feel responsive, not laggy.
4. **Long sessions** — Leave the app open for 8 hours; memory should not grow (no animation leaks).

## Anti-Patterns

Avoid these:

- ❌ **Bouncing UI** — No spring animations except toasts
- ❌ **Parallax scrolling** — Too mobile/marketing-y for a developer tool
- ❌ **Confetti / particle effects** — This is a productivity tool, not a game
- ❌ **Infinite loading bars** — Use skeletons or spinners, not misleading progress
- ❌ **Attention-grabbing flashing** — Status changes should be noticeable but not distracting
- ❌ **Long transitions (>500ms)** — Slow down user flow for no benefit
- ❌ **Animating on scroll** — Not needed in a non-scrolling grid app
