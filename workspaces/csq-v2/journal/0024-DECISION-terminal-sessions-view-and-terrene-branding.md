---
type: DECISION
date: 2026-04-11
created_at: 2026-04-11T22:10:00+08:00
author: co-authored
session_id: session-2026-04-11e
session_turn: 60
project: csq-v2
topic: Terminal Sessions view lands with a cross-platform csq-core::sessions module, targeted-swap Tauri command, and a Terrene-branded three-color tray icon set sourced from the Foundation TF monogram
phase: implement
tags: [desktop, sessions, tray, icons, terrene-branding, multi-platform]
---

# DECISION: Sessions view + targeted swap + Terrene-branded tray icons

## Context

`REMAINING-GAPS-2026-04-11.md` §3.2 (tray icon variants) and §4 (terminal sessions view) were the two largest unblocked gaps after PR #66 merged. The user has 8 accounts across ~15 terminal windows; when terminal #5 hits a 5h ceiling they need to:

1. **See** that it was terminal #5 (today: invisible — the tray quick-swap heuristic picks whatever config-N was most recently modified, which is usually the wrong one).
2. **Act** on only that terminal (today: tray swap retargets every live config-N).

Separately, §3.2 wanted colored tray icon variants for expiring / out-of-quota. Journal 0023 deferred this because generating new PNG assets required design decisions the session couldn't answer alone. For this session, the user supplied the design decision by pointing at the Terrene Foundation website repo: "Use the Terrene Foundation icon in ~/repos/terrene/contrib/website, adapt it accordingly."

## Choice

Three coordinated deliveries:

### 1. `csq_core::sessions` module — cross-platform live-process discovery

New module at `csq-core/src/sessions/{mod,macos,linux,windows}.rs`:

- **macOS** — `ps -E -o pid=,command=` dumps the environ inline; we split the command from the env blob with a heuristic that looks for ` KEY=` tokens where KEY matches `[A-Z_][A-Z0-9_]*=`. `lsof -a -p <pid> -d cwd -Fn` returns the cwd (macOS omits `ps -o cwd=` for non-Console sessions). Start time via `ps -o etimes=` minus `now`.
- **Linux** — `/proc/<pid>/comm` filters to `claude` binaries; `/proc/<pid>/environ` (NUL-separated) gives the env; `/proc/<pid>/cwd` readlink gives the cwd; `/proc/<pid>/stat` field 22 + `/proc/stat btime` gives the start time. Assumes `sysconf(_SC_CLK_TCK) == 100` to avoid a libc dep.
- **Windows** — stub returning an empty vector. Reading another process's environment on Windows needs `NtQueryInformationProcess(ProcessBasicInformation)` + `PEB` walking + `ReadProcessMemory`, which is unsafe code with Win10/11 layout gating that I can't validate autonomously. Deferred with a clear TODO.

Filter: `SessionInfo` is emitted **only** for processes whose argv\[0\] basename is `claude` **and** whose env contains `CLAUDE_CONFIG_DIR`. This drops child processes that inherit `CLAUDE_CONFIG_DIR` from a parent claude process (pyright-langserver, node MCP servers, etc.) — we want one row per top-level terminal.

**Live smoke test on the author's machine**: 14 CC sessions discovered, each with correct cwd + config dir + account id. No false positives from the inherited-env children. Result:

```
PID 37459  cwd=.../claude-squad          config=.../config-8  acct=Some(8)
PID 50949  cwd=.../aegis                 config=.../config-2  acct=Some(2)
PID 82795  cwd=.../kailash-py            config=.../config-8  acct=Some(8)
... (11 more)
```

### 2. `list_sessions` + `swap_session` Tauri commands + `SessionList.svelte` tab

`list_sessions` calls `sessions::list()`, then cross-references each row with `discovery::discover_all` + `quota_state::load_state` so the dashboard shows the **current** active account + 5h quota for each config dir — not the account the terminal launched with. If the daemon rotated `config-5` from account 2 to account 3 while terminal #5 was running, the row shows account 3, which is what the user needs to see.

`swap_session` is the targeted swap: it accepts `(base_dir, config_dir, target_account)` and retargets exactly that config dir, bypassing `most_recent_config_dir`. Path-traversal defense: `fs::canonicalize` both paths and refuse any `config_dir` that isn't a direct child of `base_dir`; second defense on the `config-<1..=999>` dir-name regex.

`SessionList.svelte` renders one row per session with cwd, `config-N`, current account + label, 5h quota percentage, and a Swap ▾ dropdown that opens an inline account picker. Polls every 5s. Mounted in a new tab system in `App.svelte` alongside the existing Accounts view; default tab is Accounts so a fresh install doesn't land on an empty Sessions view.

### 3. Three-color Terrene-branded tray icon variants

Source: `/Users/esperie/repos/terrene/contrib/website/public/favicon.svg` — the Terrene Foundation TF monogram. Via `cairosvg` + Pillow, rasterized into six PNGs:

| File                      | Size  | Color           | Template mode            |
| ------------------------- | ----- | --------------- | ------------------------ |
| `tray-normal.png` + `@2x` | 32/64 | `#FFFFFF`       | yes (macOS auto-inverts) |
| `tray-warn.png` + `@2x`   | 32/64 | `#E6A000` amber | no (color is the signal) |
| `tray-error.png` + `@2x`  | 32/64 | `#D32F2F` red   | no                       |

Mapping: `TrayHealth::{Empty, Healthy}` → `Normal`; `Expiring` → `Warn`; `OutOfQuota` → `Error`. The same precedence from journal 0023 applies: out-of-quota wins over expiring.

Wiring: `refresh_tray_menu` calls `apply_tray_icon(tray, status.icon_kind())` every 30s; initial setup in `run()` also computes the live kind before `TrayIconBuilder::build` so the menu bar reflects current state from the moment the app opens. `TrayIconKind::is_template()` maps through to `tray.set_icon_as_template(bool)` which is a no-op on Linux/Windows.

App icons (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`, `icon.png`) are **also** regenerated from the same TF monogram at fill `#1C1C1A` (the source color). This replaces the placeholder Tauri icons with Foundation branding across every platform's icon surface.

## Alternatives Considered

- **Template images for all three states** — rejected. macOS template mode forces monochrome, which collapses "warn" and "error" into the same visual. The whole point of colored variants is that red is distinguishable from amber at a glance.
- **Tray icon swap via pre-built `.icns` multi-representation files** — rejected for this session. Pillow can write `.icns` but the multi-state dynamic swap still needs per-state PNGs at runtime. The retina `@2x` variant is compiled in but currently unused pending a macOS `NSImage` double-rep wiring decision — left as a TODO with a `#[allow(dead_code)]` marker on the constant.
- **Put the sessions view in a separate window** — rejected. A tab inside the existing window matches how users actually check csq state (glance, act, move on); a second window adds window-management overhead and duplicates Header state.
- **Poll sessions at the same 5s cadence as accounts** — accepted. Keeps the feedback loop tight when the user is actively debugging a stuck terminal. CPU cost is negligible (`ps -E` + 14 `lsof` calls measured at <50ms total on the author's machine).
- **Put `sessions::list` in `session::`** (singular) — rejected. The existing `session` module handles config dir isolation per terminal; overloading it with live-process enumeration would confuse "build a session" vs "list running sessions". Kept as a sibling top-level module.
- **Windows backend via the `sysinfo` crate** — considered. `sysinfo` does not expose environment variables of foreign processes without nightly-gated features, and pulling a large dependency for one platform backend is worse than a documented stub until we can validate on real Windows.
- **Swap action outside the row (top of the view)** — rejected. The whole UX is "one terminal per row, one swap per row", and separating the action from the target invites off-by-one mistakes when the list reshuffles between polls.

## Consequences

- The single most-valuable UX gap (journal 0023 §4) is closed for macOS and Linux. Windows users see an empty Sessions tab, which is still better than no tab.
- `swap_session` is a new privileged Tauri command with path-traversal defense: both `canonicalize` and `config-N` regex checks must pass. Any future change to the config dir naming must update both sides.
- `507 Rust tests` (was 486) — 16 new in `sessions::` (parser unit tests on both platforms' exact fixtures) + 5 new in `tests::icon_kind_*`.
- `22 Svelte tests` unchanged — SessionList is covered by the TypeScript type checker and the existing test patterns; no new unit tests added because the meaningful logic is in the Rust backend and the IPC shape contract.
- Tray icon swap is verified locally by unit test but **not** end-to-end tested against a live account that transitions states in real time. The three PNG constants differ from each other by byte comparison (`icon_bytes_are_non_empty_and_distinct` test) so a regression to "all icons identical" would be caught.
- `Header.svelte` had the same `homeDir()` string-concat bug that `AccountList.svelte` already fixed in journal 0021. Fixed as a drive-by because it was exactly the same pattern.
- Tauri `image-png` feature flag is now enabled — required for `Image::from_bytes` to decode PNGs at runtime.
- The `dashboard/oauth.py` legacy header added in PR #66 remains correct.

## Follow-Up

- **Windows sessions backend** — implement `NtQueryInformationProcess` + `PEB` walk when a Windows test environment is available. Tracked under M8-03.
- **`@2x` retina tray icons** — wire into a macOS `NSImage` double-representation. Currently the `@2x` PNGs are compiled in but not sent to Tauri (see `_: &[u8] = TRAY_NORMAL_PNG_2X` dead-code marker). Low priority — Tauri's current `TrayIconBuilder::icon` only takes one `Image`.
- **Session row click → focus terminal** — would require OS-level window-focus APIs (`Accessibility` on macOS, `wmctrl` on Linux) and a way to correlate PID → terminal window. Nice-to-have, non-trivial.
- **Live OAuth end-to-end** — still unverified with real accounts. No change from journal 0023.
- **MiniMax + Z.AI key paths** — still unverified with real keys.

## For Discussion

1. The sessions filter requires `argv[0]` basename to be exactly `claude`. What breaks first when Anthropic renames or aliases the CLI binary — the filter silently drops all rows, or it false-positives on a homonym? Which failure mode is more recoverable?
2. `swap_session` canonicalizes both paths and checks `config_canon.parent() == Some(base_canon)`. If we had only done the regex check on `config-N` without the canonicalize, which attack path would that miss — and does the current defense actually close it (e.g., symlink-through-a-symlink under the base dir)?
3. The tray icon swap uses `Image::from_bytes` on every 30s tick to decode the same PNG — the decode is cheap but it's still work. Would caching the decoded `Image` instances at startup be worth the complexity (static once-lock of three `Image<'static>`), or is decoding cheap enough at this cadence that the cache is premature optimization?
