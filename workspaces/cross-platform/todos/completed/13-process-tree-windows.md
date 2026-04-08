# TODO: Windows process tree walking via CreateToolhelp32Snapshot

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todo 07 (platform detection)

## What

Add `_find_cc_pid_windows()` using `ctypes` to call `CreateToolhelp32Snapshot` + `Process32First` / `Process32Next`. This is a single kernel call that returns all processes — walk the parent chain to find the CC process.

NOT `wmic` (deprecated, removed from Windows 11 24H2). NOT `subprocess.run(["powershell", ...])` (500ms startup × 20 iterations = 10 seconds).

The ctypes approach has zero startup cost and works on all Windows versions.

## Why

`_find_cc_pid()` walks up to 20 levels of the parent process tree using `ps -p PID -o ppid=,command=`. `ps` doesn't exist on Windows. The replacement must be fast (runs on every statusline render) and available on all Windows versions.

## Acceptance

- On macOS/Linux: existing `ps` code path unchanged
- On Windows: uses CreateToolhelp32Snapshot (zero external commands)
- `python3 -m py_compile rotation-engine.py` passes
