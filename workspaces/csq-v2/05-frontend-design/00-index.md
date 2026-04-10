# csq v2.0 — Frontend Design

Complete design system and component specification for the Tauri desktop app. All artifacts are ready for implementation without guesswork.

## Read Order

1. **[01-design-system.md](01-design-system.md)** — Color palette, typography, spacing, radii, shadows, motion tokens, platform adaptations
2. **[02-components.md](02-components.md)** — Component library with TypeScript props, visual states, Svelte 5 skeleton code
3. **[03-layouts.md](03-layouts.md)** — Page layouts: dashboard, account detail, settings, OAuth modal, tray popover
4. **[04-interactions.md](04-interactions.md)** — Keyboard shortcuts, mouse interactions, drag-drop, error recovery, accessibility
5. **[05-system-tray.md](05-system-tray.md)** — Tray icon states, platform behavior (macOS popover, Windows/Linux menus)
6. **[06-motion.md](06-motion.md)** — Animation timing, easing curves, per-component motion, reduce-motion fallbacks

## Design Philosophy

csq is a **developer utility**, not a consumer app. Users are watching it from the corner of their eye while coding.

Design for:

- **Glanceability** — most important info visible in <200ms
- **Quiet by default** — the app stays out of the way; only speaks up on state changes
- **Native feel** — system fonts, native dialogs, platform-appropriate tray behavior
- **Information density** — devs want data, not whitespace; pack tight but readable
- **Precision** — exact percentages, timestamps, token counts; no rounding for aesthetics

## Key Design Decisions

| Decision                          | Rationale                                                                |
| --------------------------------- | ------------------------------------------------------------------------ |
| **Dark-first**                    | Developer tools live in dark environments; light mode is secondary       |
| **Glassmorphism on macOS**        | Native feel; Linux/Windows fall back to solid backgrounds                |
| **12px base font**                | Developers are used to terminal font sizes; utility apps should be dense |
| **Cyan accent (#06d4c4)**         | Distinctive, not corporate blue; aligns with terminal aesthetics         |
| **Lucide icons**                  | Free, consistent, tree-shakeable, 1000+ icons                            |
| **No web fonts**                  | System stack only — instant render, zero network                         |
| **Tray popover on macOS**         | Richer UX than menus; allows full account cards                          |
| **Native menus on Windows/Linux** | Platform convention; avoids custom window weirdness                      |
| **Motion under 400ms**            | Devs interact fast; longer animations feel sluggish                      |
| **Spring easing only for toasts** | Attention-grabbing for notifications; everything else feels mechanical   |

## Implementation Readiness

Each component spec includes:

- TypeScript props interface
- All visual states (default, hover, active, disabled, loading, error)
- Svelte 5 skeleton code with runes (`$state`, `$derived`, `$effect`, `$props`)
- CSS using the design system tokens
- Accessibility requirements (ARIA, keyboard, focus)
- Platform-specific notes where applicable

A frontend implementer should be able to build directly from these specs without asking design questions.

## Reference Implementation Stack

- **Framework**: Svelte 5 with runes
- **TypeScript**: strict mode
- **Icons**: `lucide-svelte` (tree-shaken imports)
- **State**: Svelte runes ($state at component level, stores for cross-component)
- **IPC**: Tauri commands (`@tauri-apps/api`)
- **Build**: Vite bundled via Tauri
- **No CSS framework** — vanilla CSS with CSS variables

## Asset Requirements

Before implementation begins, prepare:

- Tray icons: `tray-normal`, `tray-warning`, `tray-error` (macOS template + full-color for Linux/Windows)
- App icon: `icon-512.png`, `icon-256.png`, `icon-128.png`, `icon.icns` (macOS), `icon.ico` (Windows)
- Favicon for settings/about pages
- Empty state illustration (key + user icon composition)

Icons can be generated from Lucide source + a small post-process script.

## Testing Checklist

Before shipping, verify:

- [ ] Dark mode renders correctly on all screens
- [ ] Light mode renders correctly (secondary but functional)
- [ ] All interactive elements have visible focus rings
- [ ] `Tab` order follows visual order
- [ ] `prefers-reduced-motion` disables all animations
- [ ] WCAG AA contrast (4.5:1 for body text, 3:1 for large text)
- [ ] Works at 480×360 minimum window size (no clipped content)
- [ ] Works at 1200×800 maximum (no awkward empty space)
- [ ] Tray icon adapts to macOS dark/light menu bar
- [ ] Tray popover dismisses on click outside and Esc
- [ ] Toast notifications dismiss after timeout
- [ ] Keyboard shortcuts work on all platforms
- [ ] Native dialogs used for file operations (not custom modals)
- [ ] 60fps animations on 2018 MacBook baseline
- [ ] Screen reader announces state changes

## Notes on Sources

These specs were drafted directly (not generated by subagent). The `uiux-designer` agent attempted this task but hit an account rate limit. The specs reflect:

- Direct reading of the v2.0 vision, functional requirements, and M10 todo list
- Svelte 5 rune patterns from `.claude/rules/svelte-patterns.md`
- Tauri integration patterns from `.claude/rules/tauri-patterns.md`
- Modern desktop app references (Ollama, Raycast, Linear, Arc menu bar)
- WCAG 2.1 AA accessibility standards
