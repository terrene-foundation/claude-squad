# TODO: Abstract file locking — named mutex on Windows, fcntl on POSIX

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todo 07 (platform detection)

## What

Replace raw `fcntl.flock()` calls with `_lock_file()` / `_unlock_file()` helpers.

- **POSIX**: Keep `fcntl.flock(fd, LOCK_EX)` (advisory, whole-file, blocks indefinitely)
- **Windows**: Use named mutex via `ctypes.windll.kernel32.CreateMutexW` (cooperative, blocks indefinitely). NOT `msvcrt.locking()` — that has wrong semantics (mandatory byte-range, 10s timeout).

Move `import fcntl` inside the POSIX branch so it doesn't fail on Windows import.

## Call sites to refactor

1. `auto_rotate()` force-mark block (lines ~774-790)
2. `update_quota()` lock block (lines ~826-843)

Both follow the pattern: open lock file → flock(LOCK_EX) → read/modify/write data file → flock(LOCK_UN) → close.

## Acceptance

- `import fcntl` only executes on POSIX (not at module top level)
- `grep -n 'fcntl' rotation-engine.py` shows only inside the POSIX branch of the helper
- `python3 -m py_compile rotation-engine.py` passes on macOS
- Lock/unlock pattern works correctly (test: two concurrent `update_quota` calls don't corrupt quota.json)
