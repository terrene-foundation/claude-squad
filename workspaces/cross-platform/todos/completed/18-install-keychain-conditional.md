# TODO: Make keychain initialization conditional on macOS

**Milestone**: 2 — Smart Installer
**File**: `install.sh`
**Blocks**: None
**Blocked by**: Todo 16 (platform detection)

## What

Skip keychain slot initialization on non-macOS. The `for n in 1 2 3 ... 7` loop that creates config directories stays, but the keychain entry creation (if any) is macOS-only.

Print platform-appropriate message:

- macOS: "Keychain entries initialized for 7 slots"
- Linux/WSL: "File-only credential storage (no keychain on Linux)"
- Git Bash: "File-only credential storage (no keychain on Windows)"

## Acceptance

- On macOS: behavior unchanged
- On Linux: no `security` command calls, no errors
- Install completes successfully on Linux
