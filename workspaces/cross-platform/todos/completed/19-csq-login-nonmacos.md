# TODO: Fix csq login credential capture on non-macOS

**Milestone**: 2 — Smart Installer
**File**: `csq` (cmd_login function, lines ~39-71)
**Blocks**: None
**Blocked by**: Todo 16 (platform detection)

## What

This is the **most critical gap** from red team. Currently `csq login N` reads credentials from macOS Keychain after `claude auth login`. On non-macOS, there is no keychain — `security find-generic-password` doesn't exist.

On Linux/WSL/Windows, `claude auth login` writes credentials directly to `$config_dir/.credentials.json`. Read from there instead.

## Current flow (macOS only)

1. Set up config dir
2. Run `claude auth login` (opens browser)
3. Read credential from keychain via `security find-generic-password`
4. Save to `credentials/N.json`

## New flow (platform-aware)

1. Set up config dir
2. Run `claude auth login` (opens browser)
3. **macOS**: read from keychain (current behavior)
4. **Non-macOS**: read from `$config_dir/.credentials.json` (CC writes it there)
5. Save to `credentials/N.json`

## Implementation

In the inline Python block (csq lines 39-71), add platform detection:

```python
import sys
if sys.platform == 'darwin':
    # existing keychain extraction via security command
else:
    # read from config_dir/.credentials.json directly
    dot_creds = os.path.join(config_dir, '.credentials.json')
    if os.path.exists(dot_creds):
        creds = open(dot_creds).read()
        # save to credentials/N.json
```

## Acceptance

- On macOS: `csq login N` works as before (keychain)
- On Linux: `csq login N` captures credentials from `.credentials.json`
- `credentials/N.json` contains valid OAuth creds after login on all platforms
- `csq run N` works after `csq login N` on Linux
