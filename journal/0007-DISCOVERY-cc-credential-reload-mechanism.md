---
type: DISCOVERY
date: 2026-04-07
created_at: 2026-04-07T16:30:00+08:00
author: co-authored
session_id: 56e0a0d5-bb6f-4bbe-a71a-dc06dac9f951
session_turn: 35
project: claude-squad
topic: Live credential rotation works without restarting Claude Code
phase: implement
tags: [oauth, credentials, rotation, cross-process]
---

## Discovery

Empirical finding: when `.credentials.json` is updated by an external process, the running Claude Code instance picks up the new credentials on its next interaction. No restart needed.

We discovered this through testing. Prior assumption was that credential rotation required a CC restart. We disproved that by writing fresh credentials to the file from outside the process, then submitting a new message in CC — the next request used the new account.

## Implication for csq

`swap_to()` already writes `.credentials.json` and the per-config-dir keychain entry. Once the file is updated, the running CC instance picks up the change on its next interaction. **No restart is needed.** The "restart CC to activate" warning that lived in `swap_to()` was based on a wrong assumption that did not hold up under empirical testing. Removed in commit ceb3a5d.

Verified live: user ran `! csq swap 7` inside a CC session running on account 2; statusline updated to 7 immediately (because we now also write `.current-account` directly), and the next API call used account 7's credentials.

## For Discussion

1. Our prior model assumed credentials had to be loaded once at startup. The empirical test invalidated that assumption. What other assumptions about external dependencies are we still operating under that haven't been tested?
2. If we had tested rotation a year ago, the `swap_to()` warning might have been correct then and wrong now — third-party behavior can change silently. What's the equivalent of a regression test for upstream behavior we don't control?
3. The user pushed back on the "restart required" warning by trying it and seeing it just worked. This empirical-first approach caught the wrong assumption faster than any reasoning could have. How do we build that habit into earlier phases of work?
