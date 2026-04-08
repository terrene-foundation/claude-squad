# TODO: Abstract keychain — macOS only, no-op elsewhere

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todo 07 (platform detection)

## What

Guard all keychain functions with `if not IS_MACOS: return None/True`.

Functions to guard:

- `_keychain_service()` — return None on non-macOS
- `keychain_account()` — return None on non-macOS
- `write_keychain(creds)` — return True (no-op success) on non-macOS

The `security` subprocess calls stay inside the macOS-only paths. No changes to macOS behavior.

## Why

On Linux/WSL/Windows, there is no macOS Keychain. CC uses file-only storage (`.credentials.json`). The keychain on macOS is a nice-to-have fallback (CC's primary read path is the file).

## Acceptance

- On macOS: keychain behavior unchanged
- `grep -n 'subprocess.*security' rotation-engine.py` — all inside IS_MACOS guards
- `python3 -m py_compile rotation-engine.py` passes
