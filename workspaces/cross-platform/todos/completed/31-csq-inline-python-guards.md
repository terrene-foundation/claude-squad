# TODO: Guard csq inline Python for cross-platform (chmod, os.replace retry, readlink)

**Milestone**: 2 — Smart Installer
**File**: `csq` (inline Python blocks + bash commands)
**Blocks**: None
**Blocked by**: Todo 16 (platform detection)

## What

Address 5 platform-specific sites in csq that are NOT covered by rotation-engine.py todos:

1. **Lines 62, 67**: `os.chmod(path, 0o600)` in login inline Python — no-op on Windows, harmless. Add comment documenting this.
2. **Line 219**: `os.chmod(tmp, stat.S_IRUSR | stat.S_IWUSR)` in back-sync inline Python — same, document.
3. **Line 220**: `os.replace(canonical, live)` in back-sync inline Python — needs retry on Windows (PermissionError if target open). Add try/retry loop matching the pattern from Todo 14.
4. **Line 229**: `chmod 600 "$config_dir/.credentials.json"` (bash) — on Git Bash/Windows this may silently fail or succeed depending on NTFS ACL. Guard with platform check: `[[ "$PLATFORM" != "git-bash" ]] && chmod 600 ...`
5. **Line 182**: `readlink "$target"` — on Windows/Git Bash, `readlink` may not work for junctions. Add fallback: check if target is a junction via `cmd //c dir /AL` or just re-create unconditionally if platform is git-bash.

## Acceptance

- csq inline Python `os.replace()` retries on PermissionError (Windows)
- chmod calls documented as no-ops on Windows
- bash chmod guarded for Git Bash
- readlink check handles junctions or skips gracefully
- `bash -n csq` passes
