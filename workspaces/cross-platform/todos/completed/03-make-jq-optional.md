# TODO: Make jq optional in installer + detect package manager

**Milestone**: 0 — Quick Fixes
**File**: `install.sh` line 24
**Blocks**: None
**Blocked by**: None

## What

1. Change jq check from hard error to warning. Core rotation works without jq; only statusline needs it.
2. Detect package manager inline (no dependency on the `suggest_install()` function from Milestone 2 — keep this self-contained).

## Current

```bash
command -v jq &>/dev/null || { err "jq not found. brew install jq"; exit 1; }
```

## Target

```bash
if ! command -v jq &>/dev/null; then
    hint="your package manager"
    command -v brew &>/dev/null && hint="brew install jq"
    command -v apt &>/dev/null && hint="sudo apt install jq"
    command -v dnf &>/dev/null && hint="sudo dnf install jq"
    warn "jq not found — statusline will not show quota. Install with: $hint"
fi
```

Self-contained package manager detection. No shared function needed.

## Acceptance

- `install.sh` completes on a system without jq (with warning, not error)
- On macOS: suggests `brew install jq`
- On Debian/Ubuntu: suggests `sudo apt install jq`
- On Fedora: suggests `sudo dnf install jq`
