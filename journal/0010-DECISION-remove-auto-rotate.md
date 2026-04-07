---
type: DECISION
date: 2026-04-07
created_at: 2026-04-07T22:30:00+08:00
author: co-authored
session_id: 47ffd8ee-c58b-48c9-8e06-2c8ffcfc0d7a
session_turn: 110
project: claude-squad
topic: Removed auto-rotate entirely; csq is now a manual tool
phase: implement
tags: [rotation, ux, race-condition, design-philosophy]
---

## Decision

Removed auto-rotate from csq entirely. The `update_quota()` function in `rotation-engine.py` no longer calls `swap_to()` when an account hits 100%. The `auto-rotate-hook.sh` UserPromptSubmit hook is now `exit 0` (kept for wiring continuity but disabled). The `auto_rotate()` function still exists in the engine and is callable via the CLI, but nothing in csq invokes it.

Users now manually swap with `! csq swap N` when they hit a rate limit. csq's job is to make swapping fast and visible (statusline shows quota across all accounts), not to make decisions.

## Rationale

Auto-rotate was structurally unsound. It triggered on `rate_limits` data extracted from CC's statusline JSON. The problem: after a manual swap, CC's statusline JSON still contains `rate_limits` from the **previous** account's last API call (CC hasn't made a request on the new account yet, so it has no fresh data to report). `update_quota()` would attribute that stale data to the new account, see "100% used", and immediately swap again — picking yet another account. The cycle repeated across every statusline render and every active terminal, scrambling which account was loaded into which config dir.

User's exact symptom: did `! csq swap 5`, then watched their terminal silently switch to 1, then 2, then 3 — all accounts that were actually maxed out.

We tried to fix the symptom with a payload-hash cursor (`.quota-cursor`) that would refuse stale data. The cursor logic worked, but I introduced a NEW bug: `swap_to()` was deleting the cursor on every swap, removing the protection. Then a separate bug: `swap_to()` writes `.credentials.json` BEFORE `.csq-account`, so a parallel statusline-fired backsync would see new credentials but the OLD account marker and copy tokens to the wrong canonical slot.

These bugs were all compounding because auto-rotate was firing constantly. Removing auto-rotate eliminated the entire cascade.

## Alternatives Considered

1. **Fix the cursor logic perfectly** — possible but fragile. Multiple processes (statusline, backsync, swap_to) all racing on the same per-config-dir state files. Each new fix introduced another edge case.
2. **Make rate_limits source-of-truth-aware** — add an explicit account ID field to the data we cache. CC's JSON doesn't include this; we'd have to derive it from the credentials file mtime, which is unreliable.
3. **Disable auto-rotate only after a manual swap (60s pause)** — partial fix; doesn't help when CC's autocompact or background fork triggers auto-rotate at the wrong moment.
4. **Remove auto-rotate entirely (chosen)** — eliminates the whole class of bug. The user is the source of truth: they read the statusline and decide.

## Consequences

- **No more random switching.** The user's terminal stays on the account they put it on until they explicitly say otherwise.
- **One less feature.** Users who relied on hands-off rotation now have to type `! csq swap N` when they hit a limit. The statusline shows them which account has capacity, so this is one keystroke + one number.
- **Auto-rotate-hook.sh is dead code.** Kept the file as a no-op so a future re-enable is a one-line change to the script rather than rewriting `settings.json`.
- **The cursor mechanism still has value** for `update_quota()` correctness — it prevents stale rate_limits from one account being attributed to another in the quota.json display, even without auto-rotate firing on it.

## For Discussion

1. Is "automation that fires on stale data" a class of bug worth a permanent rule? csq is the second time I've seen this — the first was the OAuth refresh thundering herd (csq + CC both trying to refresh the same token). The pattern: a system tries to be helpful by acting on data, but the data lags reality, so the action makes things worse.
2. Manual `csq swap N` requires the user to type a number. Should the statusline make this easier — e.g., show `[swap: 4]` next to whichever account has the most quota, so the user can copy-paste the suggestion? Or is "the user picks" the whole point and any suggestion would drift back toward auto-rotate?
3. The auto-rotate function is still in the engine codebase (`auto_rotate()`). Should it be deleted, or kept as dead code in case we figure out a way to make it correct? Dead code rots; deleted code is hard to bring back. What's the right tradeoff?
