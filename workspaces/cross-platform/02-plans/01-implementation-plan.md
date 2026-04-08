# Implementation Plan — Cross-Platform Support

## Phase 1: rotation-engine.py cross-platform

### 1.1 Platform detection helper

```python
import sys
IS_WINDOWS = sys.platform == "win32"
IS_MACOS = sys.platform == "darwin"
```

### 1.2 File locking abstraction

Replace all `fcntl.flock()` calls with:

```python
if IS_WINDOWS:
    import msvcrt
    def _lock_file(fd):
        msvcrt.locking(fd.fileno(), msvcrt.LK_LOCK, 1)
    def _unlock_file(fd):
        msvcrt.locking(fd.fileno(), msvcrt.LK_UNLCK, 1)
else:
    import fcntl
    def _lock_file(fd):
        fcntl.flock(fd, fcntl.LOCK_EX)
    def _unlock_file(fd):
        fcntl.flock(fd, fcntl.LOCK_UN)
```

3 call sites: auto_rotate force-mark (line 774), update_quota (line 826), and any future locking.

### 1.3 Keychain abstraction

```python
def read_keychain():
    if not IS_MACOS:
        return None  # file-only on non-macOS
    # existing security find-generic-password code

def write_keychain(creds):
    if not IS_MACOS:
        return True  # no-op success on non-macOS
    # existing security add-generic-password code
```

Affects: `keychain_account()`, `write_keychain()`, `_keychain_service()`.

### 1.4 PID detection

```python
def _is_pid_alive(pid):
    if IS_WINDOWS:
        import ctypes
        kernel32 = ctypes.windll.kernel32
        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
        if handle:
            kernel32.CloseHandle(handle)
            return True
        return False
    else:
        # existing os.kill(pid, 0) code
```

```python
def _find_cc_pid():
    if IS_WINDOWS:
        # Use wmic or tasklist
        r = subprocess.run(
            ["wmic", "process", "where", f"ProcessId={pid}",
             "get", "ParentProcessId,CommandLine", "/format:csv"],
            capture_output=True, text=True, timeout=2
        )
        # parse CSV output
    else:
        # existing ps -p code
```

### 1.5 Atomic rename

Replace all `tmp.rename(target)` with `os.replace(tmp, target)` — atomic on both platforms.

4 call sites in rotation-engine.py.

### 1.6 chmod guard

```python
def _secure_file(path):
    if not IS_WINDOWS:
        path.chmod(0o600)
```

Replace all `chmod(0o600)` calls. 3 sites in rotation-engine.py.

---

## Phase 2: Smart install.sh

### 2.1 Platform detection

```bash
detect_platform() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux)
            if grep -qi microsoft /proc/version 2>/dev/null; then
                echo "wsl"
            else
                echo "linux"
            fi ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows-bash" ;;
        *) echo "unknown" ;;
    esac
}
```

### 2.2 Platform-specific behavior

| Step          | macOS                         | Linux                               | WSL           |
| ------------- | ----------------------------- | ----------------------------------- | ------------- |
| jq check      | `brew install jq` hint        | `apt install jq` / `dnf install jq` | same as Linux |
| Keychain init | Yes (current)                 | Skip                                | Skip          |
| chmod         | Yes                           | Yes                                 | Yes           |
| Hook commands | `bash ~/.claude/accounts/...` | Same                                | Same          |

### 2.3 Shebang fix

Change `statusline-quota.sh` line 1 from `#!/bin/bash` to `#!/usr/bin/env bash`.

---

## Phase 3: PowerShell scripts

### 3.1 csq.ps1 (~250 lines)

Port the core logic:

- `cmd_login` — capture from `.credentials.json` (no keychain on Windows)
- `cmd_run` — set `$env:CLAUDE_CONFIG_DIR`, create junctions, invoke `claude`
- `cmd_swap` — call `python3 rotation-engine.py swap N`
- `cmd_status` — call `python3 rotation-engine.py status`

Junctions instead of symlinks:

```powershell
New-Item -ItemType Junction -Path $target -Target $source -Force
```

### 3.2 statusline-quota.ps1 (~80 lines)

Eliminate jq dependency:

```powershell
$input_data = [Console]::In.ReadToEnd() | ConvertFrom-Json
$ctx_input = $input_data.context_window.current_usage.input_tokens
```

### 3.3 auto-rotate-hook.ps1 (~15 lines)

Trivial port of the 18-line bash script.

### 3.4 install.ps1 (~120 lines)

- Check prerequisites: `claude`, `python3`
- No `jq` needed (PowerShell has `ConvertFrom-Json`)
- Copy files to `$env:USERPROFILE\.claude\accounts\`
- Patch `settings.json` with PowerShell hook commands
- Add `csq.ps1` to PATH or create `csq.cmd` wrapper

---

## Phase 4: Documentation

### 4.1 README.md

Add platform tabs:

```
## Install

### macOS / Linux / WSL
curl -sSL ... | bash

### Windows (PowerShell)
irm https://raw.githubusercontent.com/.../install.ps1 | iex
```

### 4.2 Troubleshooting per platform

- Windows symlink permissions → enable Developer Mode
- WSL credential sharing → file-only is normal
- VS Code hooks → same as CLI, no plugin needed
