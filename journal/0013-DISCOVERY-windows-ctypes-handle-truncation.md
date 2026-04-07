---
type: DISCOVERY
date: 2026-04-08
created_at: 2026-04-08T00:15:00+08:00
author: co-authored
session_id: 47ffd8ee-c58b-48c9-8e06-2c8ffcfc0d7a
session_turn: 215
project: claude-squad
topic: Windows ctypes silently truncates HANDLE return values without explicit signatures
phase: redteam
tags: [windows, ctypes, cross-platform, ci, false-positive]
---

## Discovery

Python's `ctypes` defaults `restype` to `c_int` (32 bits). Windows API handles (`HANDLE`) are pointer-sized: 64 bits on 64-bit Windows. Without explicit `restype = ctypes.c_void_p`, every `CreateMutexW`, `OpenProcess`, `CreateToolhelp32Snapshot`, etc. silently returns the lower 32 bits of the handle. The truncated handle then fails every subsequent kernel call (`WaitForSingleObject`, `CloseHandle`, etc.) without raising any Python exception — the calls just return failure codes, which the calling code rarely checks.

This is silent corruption: locks don't lock, PID checks return wrong answers, file enumerations stop early.

## How CI Missed It

The smoke test (`test-platform.sh`) ran `_lock_file` / `_unlock_file` round-trip in a single process. The truncated handle round-tripped successfully because Python doesn't validate the handle — it just passes the int back to `CloseHandle`, which fails silently. From the test's perspective, "lock acquired, lock released, no exception". CI was green on `windows-latest` for `windows-latest × Python 3.11/3.12`. The bug was structurally present but undetectable by single-threaded smoke tests.

The red team agent flagged this as the single highest-leverage fix because it turned a passing CI signal into a false signal: every Windows lock claim was unfounded.

## What Catches It

Concurrent locking test: spawn N threads each acquiring/releasing the same lock in a tight loop, increment a shared counter inside the critical section, assert the counter never exceeds 1. With a working lock, threads serialize. With a broken lock, multiple threads enter simultaneously and the counter exceeds 1.

Added as test 13 in `test-platform.sh`. After the fix it passes on Windows; before the fix it would have failed.

## The Fix

Centralize Win32 signatures at module load time:

```python
_kernel32 = ctypes.windll.kernel32
_kernel32.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_bool, ctypes.c_wchar_p]
_kernel32.CreateMutexW.restype = ctypes.c_void_p
# ... and for every other call
```

Use the centralized `_kernel32` reference (NOT `ctypes.windll.kernel32` afresh) in every call site, otherwise the new lookup re-defaults the signature.

Also: check `WaitForSingleObject` return value. `WAIT_OBJECT_0` (0) means lock acquired; anything else means failure. The original code didn't check this — even with correct signatures, a `WAIT_FAILED` would have been ignored.

## For Discussion

1. ctypes' default-to-c_int is a footgun on every 64-bit Windows. Why hasn't Python made the default `c_void_p` for HANDLE-returning functions, or at least raised a warning when restype is left unset on Windows API calls? Is there a project-level lint that catches this?
2. Our smoke test was structurally incapable of catching this class of bug. What other "single-threaded round-trip" tests give false confidence? The cursor logic in `update_quota` is similar — it's only tested by a single process, never by two concurrent statuslines. Should we systematically add a concurrent-stress version of every such test?
3. The red team caught this only because we asked specifically about "Windows-specific issues we missed" and "untested code paths". A general "find bugs" prompt would likely have skipped it. How do we make red-team prompts produce this kind of finding without having to know the answer in advance?
