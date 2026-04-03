---
name: rotate
description: "Intelligent account rotation — auto-pick best Claude account based on quota"
---

# /rotate — Account Rotation

When the user runs /rotate, force-rotate to the best available account.

## Steps

1. Run:

   ```bash
   python3 ~/.claude/accounts/rotation-engine.py auto-rotate --force
   ```

2. If output contains `[auto-rotate]` — rotation succeeded. Say "Rotated." and resume your previous task.
3. If output contains "All accounts exhausted" — say so and show the reset times.
4. If output contains "CLAUDE_CONFIG_DIR not set" — tell the user to launch with `cc <N>`.

**IMPORTANT**: On success, do NOT show status tables or quota details. Just continue working.
