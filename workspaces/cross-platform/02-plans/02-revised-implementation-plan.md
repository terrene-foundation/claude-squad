# Revised Implementation Plan (Post Red Team)

## Phase 0: Quick Fixes (pre-cross-platform, fix today)

Independent of cross-platform work. Fixes real bugs for Linux users now.

### 0.1 Fix statusline shebang

`statusline-quota.sh` line 1: `#!/bin/bash` → `#!/usr/bin/env bash`

### 0.2 Replace bc with awk

`statusline-quota.sh` lines 69-76: replace `echo "scale=1; $n / 1000000" | bc` with `awk "BEGIN{printf \"%.1fM\", $n/1000000}"`

### 0.3 Make jq optional in install.sh

Line 24: warn instead of error. Detect package manager: `brew` (macOS), `apt` (Debian), `dnf` (Fedora). Core rotation works without jq; only statusline needs it.

### 0.4 cleanup() uses \_is_pid_alive()

Line 939: replace inline `os.kill(pid, 0)` with `_is_pid_alive(pid)`.

### 0.5 Path.rename() → os.replace() (5 sites)

Lines 68, 310, 524, 588, 713: atomic rename that works on Windows when target exists.

---

## Phase 1: rotation-engine.py Cross-Platform

### 1.1 Platform detection

```python
import sys
IS_WINDOWS = sys.platform == "win32"
IS_MACOS = sys.platform == "darwin"
```

### 1.2 File locking — named mutex on Windows, fcntl on POSIX

NOT msvcrt.locking() (wrong semantics — mandatory byte-range with 10s timeout).

```python
if IS_WINDOWS:
    import ctypes
    _kernel32 = ctypes.windll.kernel32

    def _lock_file(lock_path):
        """Acquire a named mutex. Returns the handle (must pass to _unlock_file)."""
        name = "csq_" + str(lock_path).replace("\\", "_").replace("/", "_").replace(":", "_")
        handle = _kernel32.CreateMutexW(None, False, name)
        if not handle:
            return None
        _kernel32.WaitForSingleObject(handle, 0xFFFFFFFF)  # INFINITE
        return handle

    def _unlock_file(handle):
        if handle:
            _kernel32.ReleaseMutex(handle)
            _kernel32.CloseHandle(handle)
else:
    import fcntl

    def _lock_file(lock_path):
        fd = open(lock_path, "w")
        fcntl.flock(fd, fcntl.LOCK_EX)
        return fd

    def _unlock_file(fd):
        if fd:
            try:
                fcntl.flock(fd, fcntl.LOCK_UN)
                fd.close()
            except Exception:
                pass
```

Replaces 2 lock/unlock sites (auto_rotate force-mark, update_quota).

### 1.3 Keychain abstraction

```python
def read_keychain():
    if not IS_MACOS:
        return None
    # existing security find-generic-password code

def write_keychain(creds):
    if not IS_MACOS:
        return True  # no-op
    # existing security add-generic-password code
```

### 1.4 PID detection — ctypes on Windows

```python
def _is_pid_alive(pid):
    if IS_WINDOWS:
        import ctypes
        kernel32 = ctypes.windll.kernel32
        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
        if handle:
            # Check process hasn't exited (handle can be valid for recently-exited process)
            exit_code = ctypes.c_ulong()
            kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code))
            kernel32.CloseHandle(handle)
            return exit_code.value == 259  # STILL_ACTIVE
        return False
    else:
        try:
            os.kill(int(pid), 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return True
        except (ValueError, OSError):
            return False
        return True
```

`cleanup()` refactored to use `_is_pid_alive()`.

### 1.5 Process tree walking — ctypes CreateToolhelp32Snapshot on Windows

```python
def _find_cc_pid():
    if IS_WINDOWS:
        return _find_cc_pid_windows()
    else:
        # existing ps -p code (unchanged)

def _find_cc_pid_windows():
    """Walk process tree using CreateToolhelp32Snapshot. No wmic, no PowerShell."""
    import ctypes
    import ctypes.wintypes
    # ... CreateToolhelp32Snapshot + Process32First/Process32Next
    # Single API call returns all processes. Walk parent chain.
```

Zero startup cost. Works on all Windows versions.

### 1.6 Atomic rename with retry on Windows

```python
def _atomic_write(tmp_path, target_path):
    """Atomic rename with retry for Windows file-in-use conflicts."""
    for attempt in range(5):
        try:
            os.replace(tmp_path, target_path)
            return True
        except PermissionError:
            if IS_WINDOWS and attempt < 4:
                time.sleep(0.1)
                continue
            raise
    return False
```

### 1.7 chmod guard

```python
def _secure_file(path):
    if not IS_WINDOWS:
        os.chmod(path, 0o600)
```

### 1.8 Python detection for subprocess calls

```python
def _python_cmd():
    """Return the Python 3 command for this platform."""
    if IS_WINDOWS:
        for cmd in ["python3", "python", "py"]:
            try:
                r = subprocess.run([cmd, "--version"], capture_output=True, text=True, timeout=3)
                if r.returncode == 0 and "Python 3" in r.stdout:
                    return cmd
            except FileNotFoundError:
                continue
        return "python"  # fallback
    return "python3"
```

Not used within rotation-engine.py itself (it's already running as Python), but stored for shell scripts to query.

---

## Phase 2: Smart install.sh

### 2.1 Platform detection at top of install.sh

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
        MINGW*|MSYS*|CYGWIN*) echo "git-bash" ;;
        *) echo "unknown" ;;
    esac
}
PLATFORM=$(detect_platform)
```

### 2.2 Windows redirect

If PLATFORM is git-bash, print: "Windows detected. For full support, run the PowerShell installer instead." But continue anyway since bash scripts work under Git Bash.

### 2.3 Python detection function

```bash
find_python() {
    command -v python3 2>/dev/null && return
    command -v python 2>/dev/null && { python --version 2>&1 | grep -q "Python 3" && return; }
    command -v py 2>/dev/null && return
    err "Python 3 not found"
}
PYTHON=$(find_python)
```

All scripts use `$PYTHON` instead of hardcoded `python3`.

### 2.4 Package manager detection

```bash
suggest_install() {
    local pkg="$1"
    case "$PLATFORM" in
        macos) echo "brew install $pkg" ;;
        linux|wsl)
            if command -v apt 2>/dev/null; then echo "sudo apt install $pkg"
            elif command -v dnf 2>/dev/null; then echo "sudo dnf install $pkg"
            elif command -v pacman 2>/dev/null; then echo "sudo pacman -S $pkg"
            else echo "install $pkg using your package manager"
            fi ;;
    esac
}
```

### 2.5 Keychain init conditional

Skip keychain initialization on non-macOS. Only set up credential directories and file-only storage.

### 2.6 csq login platform fix

After `claude auth login`, read credentials from:

- macOS: keychain (current behavior)
- Non-macOS: `$config_dir/.credentials.json` (CC writes it there during auth)

---

## Phase 3: Test Bash Under Git Bash on Windows

**Before writing any PowerShell**, test the existing bash scripts + Phase 2 changes under Git Bash on a Windows machine (GitHub Actions `windows-latest`).

Test checklist:

- [ ] `install.sh` runs to completion
- [ ] `csq login N` captures credentials from `.credentials.json`
- [ ] `csq run N` creates config dir, starts CC
- [ ] `csq status` shows account info
- [ ] `csq swap N` swaps credentials
- [ ] `statusline-quota.sh` renders output
- [ ] `auto-rotate-hook.sh` fires and calls engine

If all pass: **skip PowerShell port.** Document that Windows users run under Git Bash (which they already have).

If blockers found: write targeted PowerShell scripts only for what breaks.

---

## Phase 3b: PowerShell Port (ONLY if Phase 3 reveals blockers)

Deferred. Only write if Git Bash testing fails. Scope TBD based on specific failures.

---

## Phase 4: Testing Infrastructure

### 4.1 GitHub Actions CI matrix

```yaml
strategy:
  matrix:
    os: [macos-latest, ubuntu-latest, windows-latest]
```

### 4.2 Smoke test script

`test-platform.sh` (bash) / `test-platform.ps1` (PowerShell):

- Python detection works
- `rotation-engine.py status` runs without error
- File locking works (two concurrent lock attempts)
- Credential file read/write works
- Atomic rename works
- Platform-specific features (keychain on macOS, junctions on Windows)

### 4.3 Symlink/junction testing on Windows

Test that junctions work for directories. Test that file copies are used where junctions don't apply. Verify no data loss on cleanup.

---

## Phase 5: Documentation

- README platform tabs (macOS / Linux / WSL / Windows)
- VS Code limitation note (hooks may not fire; core swap works)
- Troubleshooting per platform
- Windows Developer Mode recommendation for full symlink support
- Uninstall instructions per platform (especially junction cleanup on Windows)
