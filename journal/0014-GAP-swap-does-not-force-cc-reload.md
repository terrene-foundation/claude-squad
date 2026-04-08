---
type: GAP
date: 2026-04-08
created_at: 2026-04-08T01:50:00+08:00
author: co-authored
session_id: unknown
session_turn: 50
project: claude-squad
topic: csq swap writes credentials but cannot force CC to adopt them mid-session
phase: implement
tags: [oauth, credentials, swap, claude-code, cached-tokens, root-cause-unknown]
---

## The Gap

`csq swap N` writes everything it is supposed to write:

1. `<config_dir>/.credentials.json` (atomic, verified via content match)
2. `credentials/N.json` canonical (atomic)
3. `.csq-account` marker (atomic)
4. `.current-account` live marker (atomic)
5. macOS keychain entry via `security add-generic-password -U` (verified
   with a canary test — the update lands, subsequent reads return the
   new value)

Yet the user reports — reproducibly — that after `csq swap N`, the
running CC instance continues making API calls on the PREVIOUS account.
The only reliable recovery is `csq login N`, which runs the full OAuth
browser flow.

## What We Know

- csq's writes to file and keychain all succeed. Verified directly:
  - `write_keychain()` reads back the value after writing.
  - `.credentials.json` is atomically replaced via `os.replace`.
  - `credentials/N.json` and the marker files land atomically.
- `claude auth login` (which `csq login` invokes under the hood) DOES
  make the running CC adopt the new account. Something about that flow
  crosses a barrier that csq's direct writes don't.
- The earlier "in-place swap works" claim in journal/0007 was verified
  empirically once, but under conditions we didn't control for (the
  cached access token may have been near expiry at that time).

## Strongest Hypothesis (Not Yet Confirmed)

CC caches the OAuth credentials in memory on startup (or on first
successful API call). During a session it does NOT re-read
`.credentials.json` or the keychain unless its cached access token
becomes invalid — either:

- The token expires naturally (`expiresAt` in the past), or
- An API call returns 401 (token revoked or unknown).

Under this hypothesis:

- `csq swap` updates the file and keychain but CC's cached token is
  still valid for hours, so CC keeps using it. The file/keychain
  updates are invisible until CC's next token refresh cycle.
- `csq login` works because `claude auth login` is CC's own code path.
  It either (a) revokes the old access token server-side, causing the
  next API call to 401 and reload, or (b) updates CC's in-memory state
  via an IPC/process-local path that csq can't access from outside.

This hypothesis is consistent with all observed behavior but has not
been proven. To prove it we would need to either disassemble the CC
binary (2.1.92 is a Mach-O, not a JS bundle we can grep) or run a
controlled experiment against a live CC with instrumentation.

## Why the Existing "Cursor" Defense Is Not Enough

The `.quota-cursor` payload-hash check blocks ONE kind of stale update:
the very first statusline render after a swap, where CC's `rate_limits`
JSON still holds byte-identical data from the previous account. As soon
as CC makes one new API call, the payload changes (even by a single
`resets_at` tick), the hash differs, and the cursor check lets it
through. If CC has silently stayed on the old account, that first "new"
payload is still the old account's data.

## The Current Fix (Partial — Reduces Damage, Not Root Cause)

Added `live_credentials_account()` — it returns the canonical account
number whose refresh token matches the live `.credentials.json`. Used in
two places:

- **`update_quota()`**: rate_limits are attributed to the content-matched
  account, not the marker-claimed account. This prevents the user's
  original "csq 4 is using csq 5's quota" bug — the quota file no longer
  gets corrupted during a stuck swap.
- **`statusline_str()`**: still displays the marker label (`#4:jack`) so
  the user sees their intended account. When the live refresh token
  disagrees with the marker, a small `⚠` warning flag is appended
  (`#4⚠:jack 5h:36% 7d:80%`). No alarming label flip.

This fix is protective: it stops the data corruption and surfaces the
stuck state without changing CC's behavior. **It does NOT solve the
underlying problem.** The underlying problem is that csq has no way to
force a running CC to re-read `.credentials.json` or the keychain.

## Open Questions / Next Experiments

1. **Does CC watch the keychain?** Run a controlled test: take a live
   CC instance whose cached access token has >30 min of validity.
   Record which account it's burning via statusline quota deltas. Then
   `security add-generic-password -U` the keychain entry with a different
   account's creds. Observe whether the next API call uses the new
   creds. If yes, CC DOES watch the keychain and the issue is elsewhere
   (maybe our keychain write has a different blob format). If no, CC
   caches in memory and we need a different approach.

2. **Can csq revoke CC's cached access token?** Anthropic's OAuth flow
   uses `https://platform.claude.com/v1/oauth/token` for refresh.
   Revocation is typically at `/oauth/revoke`. If we can POST the old
   access token to the revoke endpoint, CC's next API call will 401
   and reload from disk. This would make `csq swap` work by FORCE.
   Needs to be tested carefully to avoid burning refresh tokens.

3. **Can csq invoke `claude auth login` non-interactively?** `claude auth
login` only supports `--email`, `--sso`, `--console` — no way to
   pre-inject a stored token. Probably a dead end without upstream
   cooperation.

4. **Does `claude auth logout` followed by `claude auth login` on a
   specific config dir work without user interaction?** Worth testing.
   If logout drops the session cleanly and login with stored creds works
   headless, csq could wrap this as a proper "force reload" command.

## What the User Should Do Right Now

Until we resolve the root cause:

- `csq swap N` is safe to run but may not take effect. Check the
  statusline: if you see a `⚠` flag after the account number, CC is
  still on a different account.
- `csq login N` is the reliable recovery. Its quota will flow to the
  right account.
- The quota file is NOT corrupted during a stuck swap anymore. That
  part is fixed.

## For Discussion

1. Is the "in-place swap works" claim in CLAUDE.md still valid under any
   conditions, or should it be removed entirely? I'd argue: remove it,
   or replace it with "swap is best-effort — the reliable path is
   `csq login N` when in doubt."

2. If the revocation-endpoint experiment works, should csq swap
   automatically call it to force CC to reload? The cost is one extra
   API call per swap; the benefit is the command would actually work.
   Downside: if the revocation endpoint is rate-limited or undocumented,
   we could brick accounts.

3. What's the right division of responsibility between csq and CC? csq
   manages multi-account state and storage; CC consumes one account per
   session. If CC added a `claude auth reload` subcommand, this whole
   problem disappears. Worth a feature request upstream.
