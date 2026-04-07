---
type: DISCOVERY
date: 2026-04-07
created_at: 2026-04-07T22:35:00+08:00
author: co-authored
session_id: 47ffd8ee-c58b-48c9-8e06-2c8ffcfc0d7a
session_turn: 95
project: claude-squad
topic: Access tokens can be in a "stuck" state — auth.status passes but inference fails
phase: implement
tags: [oauth, credentials, claude-code, debugging, recovery]
---

## Discovery

Anthropic OAuth access tokens can enter a state where they pass `claude auth status --json` (returning the correct email and user info) but fail `/v1/messages` with HTTP 429 and a generic `{"type":"error","error":{"type":"rate_limit_error","message":"Error"}}` body — no `anthropic-ratelimit-unified-*` headers. This is NOT the standard quota rate limit (which always includes those headers).

The recovery is `csq login N` — a full OAuth browser flow that issues a fresh access token + refresh token. The new token works immediately. The old token cannot be salvaged.

## Empirical Path to the Discovery

The user reported that `csq swap 4` succeeded ("Swapped to account 4") but the next message in CC returned "You've hit your limit · resets 9pm". I told them csq login wouldn't help because the credentials in `credentials/4.json` authenticated as the right email (verified via `claude auth status --json`). The user did `csq login 4` against my objection. It worked immediately.

I made the mistake five times in a row of speculating about Opus quotas, IP throttles, cached state — none of those explained the asymmetry. I did not consider that **`auth.status` and `/v1/messages` go through different authorization paths** on Anthropic's side. The endpoint returns user info from the token claims; the inference endpoint runs additional checks (abuse detection, fraud signals, account flags) that can fail independently.

## Why It Matters for csq

csq cannot detect this state from outside. We don't make API calls — CC does. The only signal we'd see is "swap succeeds but the user complains the next call fails". Possible improvements:

1. **`csq verify N`** — make a minimal `/v1/messages` call (with `max_tokens: 1`) to test the token. If it returns 429 with empty headers, the token is stuck and `csq login N` is the recovery. This adds one API call per verification, which is fine if the user explicitly invokes it.
2. **Detect rapid re-swap pattern** — if `csq swap N` is followed by another `csq swap *` within 60 seconds, suggest `csq login N` as the next step. The implication is "the previous swap didn't help, the token might be stuck".
3. **Improve the swap success message** — say "If the next message in CC says 'rate limited' for this account when the statusline shows quota, run `csq login N` to refresh the token."

I'm not implementing any of these in this session — they're follow-ups. The journal entry exists so we don't lose the discovery.

## Lesson For Me

Stop telling the user my theory is right when their working solution says otherwise. The honest answer is "I don't know; let me actually test it." Five rounds of speculation wasted their time and trust. The empirical test (`python3 -c "urllib... slot 4 token"`) was the right move; I should have done it after the second failure, not the fifth.

## For Discussion

1. Anthropic's `/v1/messages` 429 response with no rate limit headers is undocumented. Is this a fraud-detection signal? An abuse-detection throttle? A leftover from a previous quota window that should have cleared? Without their docs we can't tell.
2. The "stuck token" state seems to clear when you log in fresh. Does logging in **invalidate** the old stuck token, or just **issue a new clean one** that bypasses whatever check was failing? If the old one is invalidated, the bad-state list is finite and recoverable. If it's a per-token check, the bad state persists for old tokens forever.
3. Should csq treat a `/v1/messages` 429-with-no-headers as a different error class than a normal rate limit? The user-facing behavior is the same ("you can't make API calls right now") but the recovery is completely different (wait vs re-login).
