# TODO: Refactor cleanup() to use \_is_pid_alive()

**Milestone**: 0 — Quick Fixes
**File**: `rotation-engine.py` line 939
**Blocks**: Todo 12 (Windows PID detection)
**Blocked by**: None

## What

Replace inline `os.kill(pid, 0)` in `cleanup()` with the existing `_is_pid_alive(pid)` function.

## Current

```python
for f in ACCOUNTS_DIR.glob(".account.*"):
    try:
        pid = int(f.name.split(".")[-1])
        os.kill(pid, 0)
    except (ValueError, ProcessLookupError):
```

## Target

```python
for f in ACCOUNTS_DIR.glob(".account.*"):
    try:
        pid = int(f.name.split(".")[-1])
        if not _is_pid_alive(pid):
            f.unlink(missing_ok=True)
    except ValueError:
```

## Why

Duplicated PID logic. When `_is_pid_alive()` gets Windows support (Todo 12), cleanup() gets it for free.

## Acceptance

- `grep -n 'os.kill' rotation-engine.py` shows only the one in `_is_pid_alive()`
- `python3 -m py_compile rotation-engine.py` passes
