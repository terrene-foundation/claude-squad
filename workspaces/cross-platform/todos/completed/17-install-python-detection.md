# TODO: Add Python detection to install.sh and all bash scripts

**Milestone**: 2 — Smart Installer
**Files**: `install.sh`, `csq`, `statusline-quota.sh`, `auto-rotate-hook.sh`
**Blocks**: None
**Blocked by**: Todo 16 (platform detection)

## What

Add `find_python()` function that tries `python3`, `python` (if Python 3), `py -3`. Store result in `PYTHON` variable. Replace all hardcoded `python3` references.

On Windows (Git Bash), `python3` may not exist. The Python installer registers `python` or the `py` launcher.

## Scale

- `csq`: ~14 `python3` references
- `statusline-quota.sh`: 3 references
- `auto-rotate-hook.sh`: 3 references
- `install.sh`: 2 references

Total: ~22 replacements across 4 files.

## Approach

Define `find_python()` in each script (or source from a shared lib). Cache the result at script start.

```bash
find_python() {
    for cmd in python3 python py; do
        if command -v "$cmd" &>/dev/null; then
            if "$cmd" --version 2>&1 | grep -q "Python 3"; then
                echo "$cmd"; return
            fi
        fi
    done
    echo "python3"  # fallback, will error
}
PYTHON=$(find_python)
```

## Acceptance

- `grep -c 'python3' csq statusline-quota.sh auto-rotate-hook.sh install.sh` — only in `find_python()` itself
- All scripts use `$PYTHON` variable
- On macOS/Linux: `PYTHON=python3` (unchanged behavior)
- On Git Bash Windows with `py` launcher: `PYTHON=py`
