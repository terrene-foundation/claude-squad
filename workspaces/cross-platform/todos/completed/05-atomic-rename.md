# TODO: Replace Path.rename() with os.replace() (5 sites)

**Milestone**: 0 — Quick Fixes
**File**: `rotation-engine.py` lines 68, 310, 524, 588, 713
**Blocks**: Todo 14 (atomic rename retry on Windows)
**Blocked by**: None

## What

Replace `tmp.rename(target)` with `os.replace(tmp, target)` at all 5 call sites. `Path.rename()` raises `FileExistsError` on Windows when the target exists. `os.replace()` handles this correctly.

## Sites

1. Line 68: `_save()` — quota/profiles file writes
2. Line 310: `write_csq_account_marker()`
3. Line 524: `refresh_token()` — credential file update
4. Line 588: `write_credentials_file()`
5. Line 713: `swap_to()` — .current-account update

## Acceptance

- `grep -n '\.rename(' rotation-engine.py` returns 0 matches
- `grep -n 'os.replace' rotation-engine.py` returns 5 matches
- `python3 -m py_compile rotation-engine.py` passes
- Existing macOS behavior unchanged (os.replace is identical to rename on POSIX)
