# Critical Platform Research Findings

## F1: os.kill(pid, 0) fails on Windows Python < 3.13 (CRITICAL)

On Windows, `os.kill(pid, 0)` raises `ValueError: Unsupported signal: 0` on Python <= 3.12. Signal 0 support was only added in CPython 3.13 via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`.

**Impact**: `_is_pid_alive()` (line 171) and `cleanup()` (line 940) will crash on Windows with Python < 3.13. Most Windows users have Python 3.11-3.12.

**Fix**: Use `ctypes.windll.kernel32.OpenProcess(0x1000, False, pid)` on Windows. Check if handle is non-zero, then `CloseHandle`. No external dependency needed.

## F2: VS Code hooks are unreliable (HIGH)

Multiple confirmed bugs:

- Issue #18547: Plugin hooks registered but not firing in VS Code
- Issue #16114: Notification hooks not working in VS Code
- Issue #28774: `permission_prompt` and `idle_prompt` hooks never fire in VS Code
- Issue #21736: Feature request for proper hooks support

`UserPromptSubmit` appears more reliable than Notification hooks but is not definitively confirmed to work in all VS Code scenarios.

**Impact**: Auto-rotate hook and statusline may not work in VS Code. Cannot guarantee VS Code parity.

**Fix**: Document VS Code limitations. Recommend CLI for full feature support. The core swap functionality (`! csq swap N`) works regardless of hooks since it's a shell command.

## F3: CC uses ~/.claude/ on ALL platforms (CONFIRMED)

| Platform  | Path                                    | Confirmed By                   |
| --------- | --------------------------------------- | ------------------------------ |
| macOS     | `~/.claude/`                            | Current implementation         |
| Linux/WSL | `~/.claude/`                            | CC docs + issues #1414, #10039 |
| Windows   | `%USERPROFILE%\.claude\` = `~/.claude/` | CC docs, issue #29049          |

Python's `Path.home() / ".claude"` resolves correctly everywhere. No `%APPDATA%` logic needed.

## F4: macOS deletes .credentials.json (MEDIUM)

Issue #1414/#10039: macOS CC deletes `.credentials.json` because it prefers Keychain. This breaks shared-home setups where Linux expects the file to exist.

**Impact for csq**: On macOS, CC may delete `.credentials.json` that csq wrote. Our keychain write is the backup. On non-macOS, the file persists.

**Fix**: Already handled — macOS writes both keychain + file. The keychain is the durable store on macOS; the file is the durable store on non-macOS.

## F5: wmic is deprecated on Windows 11 (MEDIUM)

Microsoft deprecated `wmic` in Windows 11. Replacement: `Get-CimInstance` in PowerShell or `tasklist /FI "PID eq N"` from cmd.

**Impact**: `_find_cc_pid()` Windows implementation cannot rely on wmic.

**Fix**: Use `tasklist /FI "PID eq N" /FO CSV /NH` which works on all Windows versions. Or use PowerShell `Get-Process -Id N` via subprocess.

## F6: msvcrt.locking() has different semantics (MEDIUM)

`fcntl.flock()`: advisory, whole-file, blocks until lock acquired.
`msvcrt.locking(fd, LK_LOCK, nbytes)`: mandatory, byte-range, blocks for ~10 seconds then raises IOError.

**Impact**: Our locking pattern (lock, read, modify, write, unlock) works with both, but the 10-second timeout on Windows means a busy-wait scenario could raise an error instead of blocking indefinitely.

**Fix**: Wrap msvcrt.locking in a retry loop (3 attempts, 1s backoff). Or accept the 10s timeout as sufficient — our lock hold time is milliseconds.
