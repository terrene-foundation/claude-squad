# Windows PowerShell Port Requirements

## Hook Registration

Claude Code hooks use `"type": "command"` with a shell command string:

- Default shell on Windows is **bash** (via Git for Windows, which CC requires)
- Can set `"shell": "powershell"` per hook entry for PowerShell execution
- `.ps1` scripts invoked as `powershell.exe -ExecutionPolicy Bypass -File path\to\hook.ps1`

**Decision: provide both bash and PowerShell hook scripts.** Windows users with Git Bash (standard CC install) can use the bash hooks. Windows users preferring PowerShell native get `.ps1` variants. The installer detects which to register.

## Bash → PowerShell Translation

| Bash                          | PowerShell                                                        |
| ----------------------------- | ----------------------------------------------------------------- | -------------------------------- |
| `set -euo pipefail`           | `$ErrorActionPreference = 'Stop'; Set-StrictMode -Version Latest` |
| `[[ "$n" =~ ^regex$ ]]`       | `$n -match '^regex$'`                                             |
| `array+=("item")`             | `$array += @("item")`                                             |
| `${var:-default}`             | `if (-not $var) { $default }`                                     |
| `case/esac`                   | `switch`                                                          |
| `cat <<'HEREDOC'`             | `@'...'@` here-string                                             |
| `exec env VAR=val cmd`        | `$env:VAR = $val; & cmd; exit $LASTEXITCODE`                      |
| `ln -s source target`         | `New-Item -ItemType SymbolicLink` (needs admin/DevMode)           |
| `chmod 600 file`              | No-op (NTFS profile dirs are user-private)                        |
| `readlink path`               | `(Get-Item path).Target`                                          |
| `jq -r '.field'`              | `($json                                                           | ConvertFrom-Json).field`         |
| `echo "scale=1; $n / 1000000" | bc`                                                               | `[math]::Round($n / 1000000, 1)` |
| `command -v tool`             | `Get-Command tool -ErrorAction SilentlyContinue`                  |
| `curl -sfL url -o file`       | `Invoke-WebRequest -Uri url -OutFile file`                        |

## Symlink Concern on Windows

`csq run N` creates symlinks from `config-N/` to `~/.claude/` for shared artifacts.

On Windows, symlinks require either:

- **Administrator privileges**, or
- **Developer Mode** enabled (Settings → Developer Settings → Developer Mode)

Alternatives:

1. **Directory junctions** (`New-Item -ItemType Junction`) — work without admin for directories
2. **File copies** — heavier but no privilege requirements
3. **Detect and fallback** — try symlink, fall back to junction, fall back to copy

**Decision: use junctions for directories on Windows.** Junctions work without admin, cover the directory symlink case (which is all we need — we symlink dirs like `projects/`, `sessions/`, etc.). For the rare file symlink, copy is acceptable.

## fcntl Replacement

`fcntl.flock()` is used for concurrent file locking (quota.json updates from multiple terminals).

Options:

1. **`msvcrt.locking()`** — stdlib, Windows only, byte-range locking (different semantics)
2. **`portalocker`** — pip package, cross-platform, wraps fcntl/msvcrt cleanly
3. **Platform conditional** — `fcntl` on POSIX, `msvcrt` on Windows

**Decision: platform conditional with stdlib only.** No new pip dependencies. `fcntl.flock()` on POSIX, `msvcrt.locking()` on Windows. Wrap in a `_lock_file()` / `_unlock_file()` helper.

## PID Detection

`os.kill(pid, 0)` for process liveness check:

- **macOS/Linux**: Works (signal 0 = "can I signal this process?")
- **Windows**: `os.kill(pid, 0)` calls `OpenProcess` — works in Python 3.2+ but may behave differently

`ps -p PID -o ppid=,command=` for process tree walking:

- **Windows**: No `ps`. Use `wmic process where ProcessId=PID get ParentProcessId,CommandLine /format:csv` or `tasklist /FI "PID eq N"`.

**Decision: abstract into `_is_pid_alive()` and `_find_cc_pid()` with platform branches.** These functions already exist — just add Windows paths inside them.

## Performance Note

PowerShell startup: 300-500ms vs bash: 10-50ms. For hooks that fire on every prompt (statusline), this is noticeable. However, the statusline hook calls `python3` which dominates the time budget anyway. Net impact: negligible.
