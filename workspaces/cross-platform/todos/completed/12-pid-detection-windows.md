# TODO: Windows PID detection via ctypes

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todos 04 (cleanup refactor), 07 (platform detection)

## What

Add Windows branch to `_is_pid_alive()` using `ctypes.windll.kernel32.OpenProcess` + `GetExitCodeProcess`.

`os.kill(pid, 0)` raises `ValueError` on Windows Python < 3.13. The ctypes approach works on all Python 3 versions.

```python
def _is_pid_alive(pid):
    if IS_WINDOWS:
        kernel32 = ctypes.windll.kernel32
        handle = kernel32.OpenProcess(0x1000, False, int(pid))  # PROCESS_QUERY_LIMITED_INFORMATION
        if handle:
            exit_code = ctypes.c_ulong()
            kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code))
            kernel32.CloseHandle(handle)
            return exit_code.value == 259  # STILL_ACTIVE
        return False
    else:
        # existing os.kill(pid, 0) code unchanged
```

## Acceptance

- On macOS/Linux: behavior unchanged
- `python3 -m py_compile rotation-engine.py` passes
- No `os.kill(pid, 0)` outside of the POSIX branch of `_is_pid_alive()`
