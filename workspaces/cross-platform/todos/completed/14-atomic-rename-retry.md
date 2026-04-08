# TODO: Atomic rename with retry for Windows file-in-use conflicts

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todos 05 (os.replace), 07 (platform detection)

## What

Wrap `os.replace()` calls in a `_atomic_replace(tmp, target)` helper that retries on `PermissionError` (Windows-only). On POSIX, `os.replace()` is atomic even if the target is open. On Windows (NTFS), it fails if the target is open by another process.

```python
def _atomic_replace(tmp_path, target_path):
    for attempt in range(5):
        try:
            os.replace(str(tmp_path), str(target_path))
            return
        except PermissionError:
            if IS_WINDOWS and attempt < 4:
                time.sleep(0.1)
                continue
            raise
```

Replace all 5 `os.replace()` calls (from Todo 05) with `_atomic_replace()`.

## Acceptance

- `python3 -m py_compile rotation-engine.py` passes
- On macOS: no retry (PermissionError not raised for same-volume replaces)
- On Windows: retries up to 5 times with 100ms backoff
