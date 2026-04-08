# TODO: Add platform detection to install.sh

**Milestone**: 2 — Smart Installer
**File**: `install.sh`
**Blocks**: Todos 17-20
**Blocked by**: Milestone 0

## What

Add `detect_platform()` function at top of install.sh that returns `macos`, `linux`, `wsl`, or `git-bash`. Set `PLATFORM` variable used by all subsequent logic.

Also add Windows redirect: if MINGW/MSYS/CYGWIN detected, print a note that Git Bash works but PowerShell installer is also available. Continue execution (don't block).

## Detection Logic

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
```

## Acceptance

- `PLATFORM` variable set correctly on macOS, Linux, WSL
- On Git Bash: prints info message, continues
- On unknown platform: warns and continues
