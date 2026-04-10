# csq v2.0 — System Tray Design

The system tray is the primary surface for csq — the dashboard window is a secondary detail view. Users interact with the tray 95% of the time.

## Tray Icon States

The tray icon visually reflects the worst-case state across all accounts. Three states:

| State       | Icon                                  | When                                                |
| ----------- | ------------------------------------- | --------------------------------------------------- |
| **Normal**  | `zap` (accent color)                  | All active accounts <80% quota, all tokens valid    |
| **Warning** | `zap` (amber color) + small dot badge | Any account 80-99%, OR any token expiring within 1h |
| **Error**   | `zap` (red color) + ! badge           | Any account at 100%, OR any LOGIN-NEEDED            |

### macOS Template Images

On macOS, use template images that adapt to dark/light menu bars:

```
tray-normal-template@2x.png   (32×32, black on transparent)
tray-warning-template@2x.png  (with dot in bottom-right)
tray-error-template@2x.png    (with exclamation in bottom-right)
```

The template flag tells macOS to render in the menu bar foreground color (white on dark menu bar, black on light). The colored dot/exclamation overlay is baked into the template and adapts automatically.

### Linux/Windows Full-Color Icons

On Linux (AppIndicator) and Windows (system tray), use full-color icons since menu bars don't follow the same tinting rules:

```
tray-normal-32.png    (accent cyan)
tray-warning-32.png   (amber)
tray-error-32.png     (red)
```

## Platform-Specific Behavior

### macOS — Menu Bar Popover (preferred)

Left click: show custom popover (360×480 Svelte view)
Right click: show native menu (quick actions only)

The popover is a better UX for rich content (cards, quota bars). The native menu is a fallback for accessibility and keyboard users.

**Popover behavior**:

- Positioned below the tray icon, anchored to the right edge
- Automatic dismiss on click outside, Esc, or focus loss
- Smooth open/close animation (200ms scale + opacity)
- Fixed 360×480 size, no resize

**Implementation**: Use Tauri `tray::TrayIcon::set_icon_as_template(true)` + custom window created via `WebviewWindowBuilder` with `decorations: false`, `transparent: true`, `resizable: false`.

### Windows — System Tray Menu

Windows tray apps traditionally use native menus, not popovers. Use the native Tauri menu API:

```rust
// Rebuild on every state change
let menu = Menu::with_items(&app, &[
    &PredefinedMenuItem::section_header("Active: #3 team@example.com"),
    &MenuItem::with_id(&app, "status", "5h 62% · 7d 28% · ✓ Valid", false, None)?,
    &PredefinedMenuItem::separator(&app)?,

    &Submenu::with_items(&app, "Switch Account", true, &[
        &MenuItem::with_id(&app, "swap-1", "#1 other@example.com (14%)", true, None)?,
        &MenuItem::with_id(&app, "swap-2", "#2 dev@example.com (8%)", true, None)?,
        // ...
    ])?,

    &PredefinedMenuItem::separator(&app)?,
    &MenuItem::with_id(&app, "dashboard", "Open Dashboard", true, Some("Cmd+O"))?,
    &MenuItem::with_id(&app, "refresh", "Refresh All", true, Some("Cmd+R"))?,
    &MenuItem::with_id(&app, "prefs", "Preferences...", true, Some("Cmd+,"))?,
    &PredefinedMenuItem::separator(&app)?,
    &MenuItem::with_id(&app, "quit", "Quit csq", true, Some("Cmd+Q"))?,
])?;
```

### Linux — AppIndicator

Similar to Windows menu, but uses GTK AppIndicator. Tauri handles the platform abstraction — same menu code works.

## Popover Content (macOS)

```
┌─── 360×480 ──────────────────┐
│                              │
│ csq                    ⚙  ┈  │  ← header: title + settings + resize handle
│                              │
│ ──────────────────────────── │
│                              │
│  ACTIVE                      │  ← section label (uppercase, tiny)
│                              │
│ ┌──────────────────────────┐ │
│ │ ⚡ #3 team@example.com    │ │  ← active account card (emphasized)
│ │    max · 20x             │ │
│ │                          │ │
│ │ 5h ████████░░ 62%  2h14m │ │
│ │ 7d ███░░░░░░░ 28%  4d 6h │ │
│ │                          │ │
│ │ ✓ Token valid · 2h 14m   │ │
│ │                          │ │
│ │ [ Open Dashboard ]       │ │
│ └──────────────────────────┘ │
│                              │
│ ──────────────────────────── │
│                              │
│  SWITCH ACCOUNT              │
│                              │
│  ○ #1 other@ex.com    14%  →│  ← compact rows
│  ○ #2 dev@ex.com       8%  →│
│  ○ #5 ops@ex.com      91% ⚠│
│  ○ #7 admin@ex.com   100% ⛔│
│                              │
│ ──────────────────────────── │
│                              │
│   Refresh All   ·    Quit    │  ← footer actions
│                              │
└──────────────────────────────┘
```

### Header (40px)

```
┌──────────────────────────────┐
│ csq              ⚙ Settings  │
└──────────────────────────────┘
```

- App name on the left (12px, `--text-tertiary`)
- Settings gear on the right (14px icon, opens settings window)
- Bottom border `--border-subtle`

### Active Account Card

Full `AccountCard` component (see `02-components.md`) without actions fade-in — actions always visible in the popover. Takes ~160px of height.

### Switch Account List

Compact rows (28px each):

```
○ #1 other@ex.com    14%  →
```

- Radio dot (clickable target)
- Account number + email (truncated with ellipsis if too long)
- Quota percentage (right-aligned, tabular)
- Arrow `→` indicating click action

Hover: background shifts to `--bg-hover`, cursor pointer. Click swaps to that account and updates the icon state immediately.

### Footer (40px)

```
┌──────────────────────────────┐
│   Refresh All   ·    Quit    │
└──────────────────────────────┘
```

Two ghost buttons separated by a dot divider. 36px height. Top border `--border-subtle`.

## Dynamic Icon Updates

The tray icon state must update automatically when backend state changes:

```rust
// src-tauri/src/daemon/tray_state.rs
pub struct TrayState {
    app: AppHandle,
}

impl TrayState {
    pub fn update(&self, state: &GlobalState) {
        let icon_state = compute_icon_state(state);
        let icon_path = match icon_state {
            IconState::Normal => "tray-normal",
            IconState::Warning => "tray-warning",
            IconState::Error => "tray-error",
        };

        if let Some(tray) = self.app.tray_by_id("main") {
            let _ = tray.set_icon(Some(load_icon(icon_path)));
            let _ = tray.set_tooltip(Some(&format!(
                "csq — #{} {} · {}%",
                state.active.id, state.active.label, state.active.five_hour_pct
            )));
        }
    }
}

fn compute_icon_state(state: &GlobalState) -> IconState {
    for account in &state.accounts {
        if account.token_status == TokenStatus::LoginNeeded {
            return IconState::Error;
        }
        if account.five_hour_pct >= 100.0 {
            return IconState::Error;
        }
    }
    for account in &state.accounts {
        if account.five_hour_pct >= 80.0 {
            return IconState::Warning;
        }
        if account.token_expiring_soon {
            return IconState::Warning;
        }
    }
    IconState::Normal
}
```

Update frequency: on every backend state change (debounced 500ms to avoid flicker).

## Tooltip (hover text)

On macOS/Linux, hovering the tray icon shows a tooltip:

```
csq — #3 team@example.com · 62%
```

Format:

- App name
- Active account identifier
- Primary quota percentage (5h window)

Keep short — tooltips are truncated by the OS after ~40 chars on some platforms.

## Click Handlers

### macOS

| Event         | Action                                             |
| ------------- | -------------------------------------------------- |
| Left click    | Toggle popover (open if closed, close if open)     |
| Right click   | Show native menu (keyboard/accessibility fallback) |
| Option-click  | Show native menu (alternate)                       |
| Command-click | Open dashboard window directly                     |

### Windows

| Event        | Action                        |
| ------------ | ----------------------------- |
| Left click   | Show tray menu                |
| Double click | Open dashboard window         |
| Right click  | Show tray menu (same as left) |

### Linux

| Event        | Action                 |
| ------------ | ---------------------- |
| Left click   | Show AppIndicator menu |
| Middle click | Open dashboard window  |
| Right click  | Show AppIndicator menu |

## Accessibility in Tray

- **Screen reader**: Set `tray.set_title(Some(&tooltip_text))` to announce state
- **Keyboard**: The native menu is fully keyboard navigable (arrows, Enter, Esc)
- **Popover keyboard support**: Tab through cards, Enter activates, Esc closes
- **High contrast**: Tray icons have enough contrast for both dark and light menu bars; test on both

## First-Run Prompt

When csq is installed and launched for the first time, the tray icon shows a small pulsing dot on the tray icon for 10 seconds to draw attention:

```
  ⚡   ← normal tray icon
  ⚡·  ← with pulsing dot during first-run
```

Clicking opens the onboarding modal (see `03-layouts.md` first-run section).

## Window Management

When the dashboard window is closed (x button), the app continues running in the tray. This is the core UX promise — csq is a background utility.

However, on first close, show a one-time notification:

> **csq is still running**
> Click the tray icon to reopen. To quit completely, use the tray menu → Quit.

Notification dismisses automatically after 5s. The app remembers this notification was shown (store in Tauri settings), never shows it again.

## Quit Behavior

Quitting should be explicit:

1. Tray menu → Quit csq → immediate quit
2. Cmd+Q in dashboard window → confirmation if refresh is in progress, otherwise quit
3. Force quit (OS-level) → daemon cleanup via Tauri drop handlers

Always run cleanup on quit:

- Save current state
- Release all file locks
- Close database/state file handles
- Clear in-memory tokens (zeroize via `secrecy`)
