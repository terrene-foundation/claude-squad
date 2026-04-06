---
name: rotate
description: "Rotate Claude account — auto-pick best, or pass /rotate N to swap to a specific account"
---

# /rotate — Account Rotation

When the user runs `/rotate`, rotate to the best available account.
When the user runs `/rotate <N>` (e.g. `/rotate 3`), swap to that specific account.

## Steps

1. Parse the command:
   - If an integer argument was given (`/rotate 3`), set `TARGET=3` and run:

     ```bash
     python3 ~/.claude/accounts/rotation-engine.py swap <N>
     ```

   - Otherwise, run the auto-rotate path:

     ```bash
     python3 ~/.claude/accounts/rotation-engine.py auto-rotate --force
     ```

2. Check the output:
   - If `CLAUDE_CONFIG_DIR` is set (started via `csq run`): the engine refreshes the target account's token and writes to this terminal's keychain entry. CC picks up the new creds on its next API call.
     - If it says "Swapped to account N" — **rotation succeeded**. Say "Rotated to account N." and resume your previous task.
     - If it says "All accounts exhausted" — say so and show the reset times.
     - For explicit `/rotate N`, only the swap line will print on success.
   - If `CLAUDE_CONFIG_DIR` is NOT set: explicit swap will fail (no keychain entry to write). Auto-rotate falls back to outputting JSON suggesting the best account.
     - Parse the JSON and tell the user to run `/login <email>` with the suggested account's email.

**IMPORTANT**: On success (auto-swap or explicit swap), do NOT show status tables. Just continue working.
