---
name: rotate
description: "Intelligent account rotation — auto-pick best Claude account based on quota"
---

# /rotate — Account Rotation

When the user runs /rotate, perform intelligent account rotation.

The user calls this when they're rate-limited. The `--force` flag marks the current account as
exhausted so the engine picks an alternative even if quota data is stale.

## Steps

1. Run the rotation engine with `--force` (the user is asking because they're rate-limited):
   ```bash
   python3 ~/.claude/accounts/rotation-engine.py auto-rotate --force
   ```

2. Check the output:
   - If it contains `[force-rotate]` or `[auto-rotate]` — **rotation succeeded**.
     Say only: "Rotated. Continuing." Then **resume your previous task immediately**. Do NOT show status, reset times, or quota tables.
   - If it contains "No accounts available" — all accounts are in cooldown. Say: "All accounts in cooldown." and show the reset times from the output.

**IMPORTANT**: When rotation succeeds, do NOT run `status`, do NOT show quota tables, do NOT discuss reset times. Just continue working.

## When to auto-suggest rotation

If you notice a rate limit error in the conversation (429, "rate limited", "usage limit"), proactively suggest: "You've hit a rate limit. Run /rotate to switch accounts."
