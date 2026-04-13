---
kind: DISCOVERY
date: 2026-04-13
---

# Anthropic OAuth token endpoint requires JSON body, not form-encoded

Empirically verified via repeated failure in the daemon's refresh
path and confirmed against Claude Code's minified `cli.js` source.

## What changed

Anthropic's `/v1/oauth/token` endpoint now rejects
`Content-Type: application/x-www-form-urlencoded` bodies with
`400 invalid_request_error`. The endpoint requires
`Content-Type: application/json` with a body shape matching
Claude Code's `vw8` (refresh) and `Tw8` (code exchange) functions:

```json
{
  "grant_type": "refresh_token",
  "refresh_token": "<token>",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
  "scope": "user:inference user:profile"
}
```

## Symptom

All 7 accounts silently entered broker-failed state at the same
second. Per-account cooldowns piled up. Dashboard showed
"recovery failed — re-login needed" for every account.

## Impact

- `csq-core/src/credentials/refresh.rs` → rewritten to emit JSON
- `csq-core/src/oauth/exchange.rs` → same (login flow)
- All callers switched from `http::post_form*` to `http::post_json`
- `http::post_form_params` removed (no callers remain)

## Related

- `BrokerResult::RateLimited` added to distinguish 429-driven
  skips from lock-contention skips
- Retry-in-loop removed from `refresh_token` and `exchange_code` —
  amplified rate-limit cascades

See commit `8a9fdc9` (JSON body + rate-limit aware broker) and
`cef4b1e` (round-1 red team fixes).
