# TODO: Guard chmod calls — no-op on Windows

**Milestone**: 1 — Engine Cross-Platform
**File**: `rotation-engine.py`
**Blocks**: None
**Blocked by**: Todo 07 (platform detection)

## What

Create `_secure_file(path)` helper that calls `os.chmod(path, 0o600)` only on POSIX. Replace all direct `chmod` calls.

## Sites (3 in rotation-engine.py)

1. Line ~67: `_save()` tmp file
2. Line ~522: `refresh_token()` credential file
3. Line ~587: `write_credentials_file()` credential file

## Why

`os.chmod()` with POSIX permission bits is a no-op on Windows. Harmless but meaningless — and if we later add Windows-specific ACL protection, having a single helper makes that easy.

## Acceptance

- `grep -n 'chmod' rotation-engine.py` shows only inside `_secure_file()`
- `python3 -m py_compile rotation-engine.py` passes
