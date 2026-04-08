---
type: DISCOVERY
date: 2026-04-08
created_at: 2026-04-08T11:30:00+08:00
author: co-authored
session_id: unknown
session_turn: 30
project: claude-squad
topic: Root cause of re-login 401s — backsync silently failed when Anthropic rotated refresh tokens, leaving stale canonicals that later swaps copied back over valid live tokens
phase: implement
tags:
  [oauth, credentials, backsync, refresh-token-rotation, 401, swap, root-cause]
---

## Summary

Journal 0016's assertion that "refresh tokens are NOT rotated by Anthropic"
is empirically **wrong**. Anthropic DOES rotate refresh tokens during
CC's normal `/v1/oauth/token` refresh flow. CC's source code
(`src/services/oauth/client.ts:178`) destructures the response with a
default: `refresh_token: newRefreshToken = refreshToken` — which means
the server CAN return a new `refresh_token`, and when it does, CC
adopts it.

This invalidates a core assumption in `backsync()`, which only writes
`credentials/N.json` when the refresh token in the live
`.credentials.json` content-matches one of the canonicals. Under
rotation, the match fails silently, and `credentials/N.json` is left
frozen with the OLD refresh token. The next `csq swap N` from a
different terminal then copies that stale token back into its live
`.credentials.json`, overwriting CC's valid rotated token. CC's next
refresh attempt uses the dead token → 401 → "Please run /login".

Empirical verification: this session's live system had
`config-4/.credentials.json` holding refresh token
`sk-ant-ort01-q-GuqRU...` (rotated by CC) while `credentials/4.json`
still held `sk-ant-ort01-dhsziMp...` (the original login token).
backsync had been silently failing for this account.

## The Two Observed Symptoms (User Report)

1. **Re-login required after idling.** Idle config dir sits for hours.
   Something (another terminal's swap, or CC's own refresh) overwrites
   the rotated token with the stale canonical. On next use, 401.
2. **Re-login required mid-work.** User is actively working when CC
   pops "Please run /login". Same mechanism: another csq terminal
   swapped to this account, writing the stale canonical back over
   CC's valid rotated token.

Both symptoms have the same root cause: the stale-canonical cycle.

## The Fix

`backsync()` now has a marker fallback. If refresh-token content
matching fails, it falls back to the `.csq-account` marker (which
csq writes deterministically on every `csq run` and `csq swap`).
The fallback identifies the intended account for this config dir
and writes the live (rotated) tokens into that account's canonical.

### Safety Against Ping-Pong

When two config dirs run the same account with different OAuth
sessions (e.g., config-4 running account 4 natively + config-2
swapped to account 4 via stale canonical copy), each holds a
different refresh token. Without a guard, both would rewrite
`credentials/4.json` on every render, producing a ping-pong.

Defense: the marker fallback only writes canonical if the live
`expiresAt` is STRICTLY NEWER than the canonical's `expiresAt`.
Whichever terminal refreshed most recently wins; older terminals
leave canonical alone. This converges to the freshest token.

### What It Does NOT Fix Automatically

If a config dir holds an OLDER token than the canonical (e.g.,
config-2 above), backsync's guard prevents it from overwriting
the good canonical — correct. But the config dir itself still
holds the stale token. The user must `csq swap N` in that terminal
to pull the fresh canonical into live. (Alternatively, Anthropic's
OAuth server will eventually 401 the stale token on next refresh,
and CC's 401 recovery path will re-read disk, but the config dir's
disk still has the stale token until manual swap.)

A future improvement: add a "pullsync" that, on statusline render,
updates live from canonical IF canonical is strictly newer. Skipped
for this fix because:

- Pullsync would clobber CC's in-memory access token mid-session.
  Safer to let the user trigger it via `csq swap N`.
- The marker-fallback + timestamp guard is already sufficient to
  stop the corruption cycle at its source.

## The Earlier Revert

This same session first tried a different fix: making
`update_quota()` skip data updates on any account transition
(detected via the `.quota-cursor`). That fix was reverted because
it dropped LEGITIMATE post-swap updates when renders are hours
apart — in heavy agent workflows, the first post-swap render often
contains significant accumulated quota data that must not be
discarded. The original payload-hash cursor check is restored.

The fixes address orthogonal issues:

- **update_quota cursor**: protects quota.json from cross-account
  attribution in the swap-then-render race.
- **backsync marker fallback**: protects credentials/N.json from
  going stale when Anthropic rotates refresh tokens.

The re-login symptoms were caused by the second issue, not the
first.

## For Discussion

1. Should csq add an automatic "pullsync" (live ← canonical when
   canonical.expiresAt > live.expiresAt), and if so, how do we
   avoid clobbering CC's in-memory state mid-turn? One option:
   pullsync only when `.live-pid` is dead (CC restarting), so the
   new CC process reads the fresh canonical from the start.

2. The rotation behavior contradicts journal 0016's empirical
   claim from this morning. What changed? Possibility: Anthropic
   rotates under specific conditions (e.g., after N refreshes, on
   session boundaries, during high-load periods). Worth tracking
   by instrumenting `backsync()` to log when the fallback fires
   vs content match.

3. If two config dirs legitimately run the same account
   concurrently (as observed here for account 4), the canonical
   can only hold one token. The timestamp guard picks the freshest,
   but the "losing" config dir is now holding a token that may
   have been invalidated by Anthropic's rotation policy. Should
   csq actively prevent two config dirs from running the same
   account, or support multiple canonicals per account
   (e.g., `credentials/4-primary.json`, `credentials/4-swap.json`)?
