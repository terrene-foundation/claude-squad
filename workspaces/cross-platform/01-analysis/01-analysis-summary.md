# Cross-Platform Analysis Summary

## Scope

Make claude-squad work on macOS, Linux, WSL, and Windows PowerShell with a smart installer that auto-detects the platform.

## Key Architectural Decisions

### D1: File-only credentials on non-macOS

macOS Keychain remains the primary credential store on macOS (current behavior). All other platforms use file-only storage (`credentials/N.json` + `config-N/.credentials.json`). Rationale: CC's Windows Credential Manager integration has known bugs (#29049, #33122). File-only is the reliable path that works everywhere.

### D2: No new Python dependencies

`rotation-engine.py` stays stdlib-only. File locking uses `fcntl` on POSIX and `msvcrt` on Windows via a platform conditional helper. No `portalocker`, no `keyring`, no `psutil`. These add install friction for what is a single-file tool.

### D3: Bash scripts serve macOS + Linux + WSL; PowerShell scripts serve Windows

Two parallel script sets, not one "universal" script. Bash is native on macOS/Linux/WSL. PowerShell is native on Windows. The installer detects which platform and installs the right set.

### D4: Symlinks → junctions on Windows

`csq run N` creates symlinks for shared artifacts. On Windows, directory junctions (`mklink /J`) work without admin privileges. File symlinks fall back to copies.

### D5: Smart installer = two entry points

`install.sh` (bash) for macOS/Linux/WSL. `install.ps1` (PowerShell) for Windows. Each auto-detects the sub-platform and adjusts behavior. Not one script trying to be universal.

## Work Breakdown

### Phase 1: rotation-engine.py cross-platform (foundation)

Abstract 3 platform-specific concerns behind helpers:

1. **File locking**: `_lock_file()` / `_unlock_file()` — fcntl on POSIX, msvcrt on Windows
2. **Keychain**: skip on non-macOS (file-only); keep macOS keychain as best-effort
3. **PID detection**: `_is_pid_alive()` and `_find_cc_pid()` with Windows branches
4. **Atomic rename**: `os.replace()` everywhere instead of `Path.rename()`
5. **chmod**: no-op on Windows

### Phase 2: install.sh smart detection

Add platform detection to install.sh:

- macOS: current behavior (keychain init, brew hints)
- Linux: skip keychain, suggest apt/dnf for jq
- WSL: same as Linux, detect via `uname -r | grep -i microsoft`

### Phase 3: PowerShell scripts (Windows native)

Port 4 scripts:

1. `csq.ps1` — CLI entry point (~200 lines, most logic is arg parsing + calling python3)
2. `statusline-quota.ps1` — eliminates jq dependency via `ConvertFrom-Json`
3. `auto-rotate-hook.ps1` — trivial (15 lines)
4. `install.ps1` — Windows installer with credential manager detection

### Phase 4: README + documentation

Platform-specific install instructions, troubleshooting per platform.

## Risk Assessment

| Risk                                                            | Severity | Mitigation                                       |
| --------------------------------------------------------------- | -------- | ------------------------------------------------ |
| Windows symlink/junction permissions                            | Medium   | Detect and fall back to copies                   |
| CC hooks may not fire in VS Code on Windows                     | Medium   | Document; users can run CLI directly             |
| `os.kill(pid, 0)` on Windows kills the process                  | High     | Use `ctypes.windll.kernel32.OpenProcess` instead |
| PowerShell execution policy blocks .ps1                         | Medium   | Installer uses `-ExecutionPolicy Bypass`         |
| `msvcrt.locking()` has different semantics than `fcntl.flock()` | Low      | Lock entire file, not just advisory lock         |
