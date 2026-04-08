# Red Team Convergence — Analysis Phase

Two independent red team passes, 14 unique findings each with significant overlap validating the concerns.

## Blocking Findings (must fix before implementation)

### B1: msvcrt.locking() is fundamentally wrong (CRITICAL)

Both agents flagged this. `fcntl.flock()` is advisory whole-file; `msvcrt.locking()` is mandatory byte-range with 10-second timeout on empty lock files.

**Resolution**: Use Windows named mutexes via `ctypes.windll.kernel32.CreateMutexW` for cooperative locking on Windows. This is the true equivalent of advisory file locking on POSIX. No external dependency. Falls back to `fcntl.flock()` on POSIX (unchanged).

### B2: csq login has no credential capture path on non-macOS (CRITICAL)

The inline Python in csq calls `security find-generic-password` which doesn't exist on Linux/WSL/Windows. On non-macOS, CC writes credentials to `$config_dir/.credentials.json` during `claude auth login`.

**Resolution**: After `claude auth login` on non-macOS, read from `$config_dir/.credentials.json` directly and copy to `credentials/N.json`. Platform detection in the inline Python block.

### B3: python3 binary doesn't exist on Windows (HIGH)

Windows Python is `python` or `py -3`, not `python3`. Every bash script and the plan reference `python3`.

**Resolution**: Add `_find_python()` helper that tries `python3`, `python`, `py -3`. All scripts use the resolved path. Install.sh validates this at install time.

### B4: No testing strategy (HIGH)

No CI, no smoke tests, no manual protocol.

**Resolution**: GitHub Actions matrix (macos-latest, ubuntu-latest, windows-latest). Smoke test script per platform. See updated plan.

### B5: os.kill(pid, 0) + cleanup() call site missed (HIGH)

`cleanup()` at line 939 uses inline `os.kill(pid, 0)` instead of `_is_pid_alive()`. The plan only covers `_is_pid_alive()`.

**Resolution**: Refactor `cleanup()` to use `_is_pid_alive(pid)` now. Then the Windows ctypes path covers all sites automatically.

## Strategic Revision: Git Bash First, PowerShell Later

**Both red teams independently flagged the same insight**: Claude Code requires Git for Windows. Git for Windows includes bash. Therefore every Windows CC user already has bash. The existing bash scripts may work under Git Bash with minimal changes.

**New strategy**:

1. Phase 1: rotation-engine.py cross-platform (unchanged)
2. Phase 2: install.sh smart detection + bash fixes (shebang, python detection, jq optional)
3. Phase 3: **TEST bash scripts under Git Bash on Windows** — if they work, skip PowerShell port entirely
4. Phase 3b: PowerShell port ONLY if Git Bash testing reveals blockers
5. Phase 4: Documentation

This potentially eliminates ~400 lines of new PowerShell code.

## Quick Fixes (address now, independent of cross-platform work)

| Fix                                                       | File                | Lines              | Effort  |
| --------------------------------------------------------- | ------------------- | ------------------ | ------- |
| Shebang `#!/bin/bash` → `#!/usr/bin/env bash`             | statusline-quota.sh | 1                  | 1 line  |
| Replace `bc` with `awk`                                   | statusline-quota.sh | 69-76              | 3 lines |
| `brew install jq` → platform-aware hint; make jq optional | install.sh          | 24                 | 5 lines |
| `cleanup()` use `_is_pid_alive()`                         | rotation-engine.py  | 939                | 1 line  |
| `Path.rename()` → `os.replace()` (5 sites, not 4)         | rotation-engine.py  | 68,310,524,588,713 | 5 lines |

## Updated Risk Register

| Risk                                     | Severity | Status                                                                                  |
| ---------------------------------------- | -------- | --------------------------------------------------------------------------------------- |
| msvcrt.locking() semantics               | CRITICAL | Resolved: use named mutex on Windows                                                    |
| csq login on non-macOS                   | CRITICAL | Resolved: read .credentials.json directly                                               |
| python3 on Windows                       | HIGH     | Resolved: python detection helper                                                       |
| os.kill(pid, 0) on Windows               | HIGH     | Resolved: ctypes OpenProcess + all call sites                                           |
| No testing                               | HIGH     | Resolved: CI matrix + smoke tests                                                       |
| wmic deprecated                          | HIGH     | Resolved: ctypes CreateToolhelp32Snapshot                                               |
| Junctions directory-only                 | HIGH     | Resolved: junctions for dirs, copy for files, document Developer Mode for full symlinks |
| os.replace() not atomic when target open | HIGH     | Resolved: retry loop on PermissionError                                                 |
| PowerShell port scope                    | MEDIUM   | Resolved: Git Bash first strategy                                                       |
| VS Code hooks unreliable                 | MEDIUM   | Documented limitation; core swap works regardless                                       |
