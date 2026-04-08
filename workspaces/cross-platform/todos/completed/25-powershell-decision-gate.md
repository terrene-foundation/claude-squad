# TODO: Decision gate — PowerShell port needed?

**Milestone**: 3 — Windows Validation
**Status**: COMPLETED — DECISION: NOT NEEDED

## Decision

**PowerShell port is NOT needed.** Bash scripts work natively on Windows via Git Bash (which Claude Code requires anyway).

## Evidence

GitHub Actions CI run 24083444968 — `windows-latest` runner with Python 3.11 and 3.12 ran `test-platform.sh` (12 test cases, 20 PASS conditions) under Git Bash. **All 20 PASS conditions passed on Windows.**

Windows-specific code paths verified working:

- `_atomic_replace` with retry on file-in-use (Windows NTFS quirk)
- `_is_pid_alive` via `ctypes.windll.kernel32.OpenProcess + GetExitCodeProcess`
- `_find_cc_pid` via `CreateToolhelp32Snapshot + Process32FirstW/NextW`
- `_lock_file/_unlock_file` via named mutex (`CreateMutexW + WaitForSingleObject + ReleaseMutex`)

The bash scripts themselves run cleanly under Git Bash:

- `csq`, `install.sh`, `statusline-quota.sh`, `auto-rotate-hook.sh`, `test-platform.sh` all pass syntax checks
- `find_python` correctly resolves `python3` (Python 3 from setup-python action)
- All inline Python uses `$PY` instead of hardcoded `python3`

## Consequence

Todo 26 (PowerShell port) is **SKIPPED**. No PowerShell scripts required. Windows users install via the same `install.sh` from Git Bash. The README documents Git Bash as the supported Windows shell.

This eliminates ~400 lines of duplicate PowerShell code that would have needed parallel maintenance.
