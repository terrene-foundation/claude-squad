# Platform-Specific Code Inventory

## Summary

| Category                             | Count           | Files Affected                      |
| ------------------------------------ | --------------- | ----------------------------------- |
| macOS-only (keychain `security` CLI) | 6 call sites    | rotation-engine.py, csq             |
| POSIX-only (breaks on Windows)       | ~21 occurrences | rotation-engine.py, csq, install.sh |
| Bash-specific (justified by shebang) | ~37 uses        | all .sh files, csq                  |
| Hardcoded path `~/.claude/`          | 12 occurrences  | all files                           |

## macOS-Specific (6 items)

These break on ALL non-macOS platforms:

1. **`security find-generic-password`** — reads keychain (rotation-engine.py:325-329, csq:51)
2. **`security add-generic-password`** — writes keychain (rotation-engine.py:553-568)
3. **`_keychain_service()`** — computes macOS keychain service name (rotation-engine.py:532-543)
4. **`keychain_account()`** — reads account from keychain (rotation-engine.py:316-354)
5. **Inline Python in csq** calling `security` (csq:39-71)
6. **`brew install jq`** suggestion in install.sh (line 24)

## POSIX-Specific (21 items, breaks on Windows)

| What                                    | Where                                                                 | Windows Fix                                         |
| --------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------- |
| `import fcntl` + `flock()`              | rotation-engine.py:33,774-790,826-843                                 | `msvcrt.locking()` or conditional import            |
| `os.kill(pid, 0)` signal probe          | rotation-engine.py:167-178,939                                        | `ctypes.windll.kernel32.OpenProcess` or `psutil`    |
| `ps -p PID -o ppid=,command=`           | rotation-engine.py:194-199                                            | `wmic` or `tasklist` or `psutil`                    |
| `chmod 0o600`                           | rotation-engine.py:67,522,587; csq:62,67,219,229; install.sh:27,46-47 | No-op on Windows (NTFS user dirs are adequate)      |
| `Path.rename()` (non-atomic on Windows) | rotation-engine.py:68,524,588,713                                     | `os.replace()` everywhere                           |
| `ln -s` symlinks                        | csq:194                                                               | `New-Item -ItemType Junction` (needs admin/DevMode) |
| `readlink`                              | csq:182                                                               | `(Get-Item).Target` in PowerShell                   |
| `bc` command                            | statusline-quota.sh:69-76                                             | `awk` or inline Python                              |

## Hardcoded Paths (12 occurrences)

All files assume `~/.claude/accounts/`. On Windows, Claude Code uses `%USERPROFILE%\.claude\` (same structure, different expansion). Python's `Path.home() / ".claude"` resolves correctly on all platforms.

Shell scripts use `$HOME/.claude/` which expands correctly on macOS, Linux, WSL. PowerShell equivalents need `$env:USERPROFILE\.claude\`.

## External Dependencies

| Command    | Platforms    | Required?                                                           |
| ---------- | ------------ | ------------------------------------------------------------------- |
| `python3`  | All          | Yes (core engine)                                                   |
| `jq`       | macOS, Linux | Yes (statusline); PowerShell eliminates need via `ConvertFrom-Json` |
| `bc`       | macOS, Linux | Yes (statusline); replaceable with `awk`                            |
| `security` | macOS only   | Yes (keychain); platform abstraction needed                         |
| `ps`       | macOS, Linux | Yes (PID detection); `psutil` on Windows                            |
| `git`      | All          | Optional (statusline git status)                                    |
| `curl`     | macOS, Linux | Only for remote install; PowerShell uses `Invoke-WebRequest`        |
