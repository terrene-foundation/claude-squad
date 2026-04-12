# 0030 — Rename to Code Session Quota, Windows/Linux CI fix, UX improvements

**Type**: DECISION + DISCOVERY
**Date**: 2026-04-12

## Decisions

1. **Renamed "Claude Squad" to "Code Session Quota (csq)"** — all user-visible strings, repo (terrene-foundation/csq), tray, header, rules, README. GitHub auto-redirects old URLs.
2. **7d rank badges replace "next" badge** — numbered ranks (1,2,3...) on accounts by reset time. Accounts >= 99.5% usage excluded from ranking (maxed = moot).
3. **Session sort** — title and account sort modes added to Sessions tab, matching Accounts tab pattern.

## Discoveries

1. **Windows build failures** — three issues: (a) `ReadProcessMemory` in wrong module (`Win32::System::Memory` → `Win32::System::Diagnostics::Debug`), (b) `NtQueryInformationProcess` not in windows-sys (needed manual extern FFI), (c) HANDLE is `isize` not pointer (`.is_null()` → `== 0`).
2. **daemon_supervisor unconditional import** — imported `daemon::server` (unix-only module) without cfg gate. Fixed by splitting imports.
3. **needs_restart false positives** — `csq run N` writes marker before spawning claude, so marker is always newer than process. Fixed with 5-second grace period.
4. **Config-6 statusline broken** — pointed to nonexistent `~/.claude/statusline-command.sh` instead of `~/.claude/accounts/statusline-quota.sh`. Local fix only.
