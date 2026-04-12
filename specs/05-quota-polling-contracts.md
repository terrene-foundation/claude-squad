# 05 Quota Polling Contracts

Spec version: 1.0.0 | Status: DRAFT (endpoint details pending Playwright investigation) | Governs: Anthropic and third-party usage polling

---

## 5.0 Scope

This spec defines the daemon's contract with Anthropic's OAuth usage endpoint and third-party providers (MiniMax, Z.AI). It specifies the request shape, parse rules, and write invariants for `quota.json`.

**Status note:** sections 5.2 (claude.ai dashboard endpoint) and 5.3 (MiniMax), 5.4 (Z.AI) are DRAFT pending live Playwright investigation — the current assumptions predate direct inspection of the web dashboards' network traffic and may be wrong in the same way journal 0028's "utilization as fraction" was wrong. Results land in a revision bump once Playwright MCP is loaded and the investigations complete.

## 5.1 Anthropic `/api/oauth/usage`

**Request:**

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <access_token>
Anthropic-Beta: oauth-2025-04-20
Accept: application/json
User-Agent: curl/<csq-version>     (required — non-curl UAs get 400)
```

Transport constraints (journal 0028 Discovery, load-bearing):

- HTTP/1.1 only. HTTP/2 fails.
- No compression (`no_gzip/no_brotli/no_deflate`).
- `User-Agent` MUST start with `curl/`. This is a server-side allowlist; non-curl UAs return 400 "Invalid request format".

**Response shape:**

```json
{
  "five_hour": { "utilization": 42.0, "resets_at": "2026-04-12T20:00:00Z" },
  "seven_day": { "utilization": 15.0, "resets_at": "2026-04-18T00:00:00Z" }
}
```

**Parse rule (load-bearing):** `utilization` is a percentage in `[0, 100]`, NOT a fraction in `[0, 1]`. Multiplying by 100 produced the 5800% bug that spawned the entire journal 0028 cleanup. The current code in `parse_usage_response` correctly stores the value directly. The header comment on `daemon::usage_poller` is stale (still says "0.0-1.0") and MUST be corrected to avoid re-introducing the bug.

**Resolved (2026-04-12 Playwright investigation):** the 85% vs 100% discrepancy was NOT an endpoint difference. Both endpoints return the same `utilization` field on the same 0-100 scale. The stale reading was caused by the daemon poller dying at 12:17 UTC (see section 5.6). Fix the poller hang and the display matches the web.

## 5.2 claude.ai web dashboard (RESOLVED)

**Investigated 2026-04-12 via Playwright MCP.** The web dashboard at `claude.ai/settings/usage` calls a DIFFERENT endpoint from what csq uses, but the core data is equivalent.

**Endpoint:** `GET https://claude.ai/api/organizations/<org-uuid>/usage`
**Auth:** session cookie (NOT bearer token — csq cannot use this endpoint directly)
**Response:**

```json
{
  "five_hour": {
    "utilization": 8,
    "resets_at": "2026-04-12T16:00:01.287405+00:00"
  },
  "seven_day": {
    "utilization": 4,
    "resets_at": "2026-04-18T11:00:00.287430+00:00"
  },
  "seven_day_oauth_apps": null,
  "seven_day_opus": null,
  "seven_day_sonnet": { "utilization": 0, "resets_at": null },
  "seven_day_cowork": null,
  "iguana_necktie": null,
  "extra_usage": {
    "is_enabled": false,
    "monthly_limit": null,
    "used_credits": null,
    "utilization": null
  }
}
```

**Key findings:**

1. Same core fields as `/api/oauth/usage`: `five_hour.utilization`, `seven_day.utilization`, same 0-100 percentage scale.
2. Additional fields not in the bearer endpoint: per-model 7-day breakdowns (`seven_day_opus`, `seven_day_sonnet`), `seven_day_oauth_apps` (CC-specific usage), `seven_day_cowork`, `extra_usage` (overage billing).
3. Auth is session-cookie-only — csq cannot replay this without maintaining a browser session.
4. Bootstrap call (`GET /api/bootstrap/<org-uuid>/app_start`) returns `rate_limit_tier: "default_claude_max_20x"` confirming subscription tier.

**Decision:** csq stays on `/api/oauth/usage` (bearer-authenticated). The data is equivalent for the fields csq needs. The web endpoint gives richer breakdown data that csq could expose later if cookie auth becomes viable.

## 5.3 MiniMax (RESOLVED — 3 bugs found)

**Investigated 2026-04-12 via Playwright MCP.** The endpoint works and returns authoritative data. csq was calling it wrong.

**Working endpoint (from browser):**

```
GET https://platform.minimax.io/v1/api/openplatform/coding_plan/remains?GroupId=<group-id>
Authorization: Bearer <API_KEY>
Accept: application/json
```

**csq's current call (broken — `usage_poller.rs:849`):**

```
GET https://www.minimax.io/v1/api/openplatform/coding_plan/remains
Authorization: Bearer <API_KEY>
```

**Three bugs:**

1. **Wrong host.** csq uses `www.minimax.io`. The working host is `platform.minimax.io`. The `www` host returns 403 via Cloudflare. The catalog also has a third host `api.minimax.chat` for the Anthropic-compatible endpoint — that's correct for message traffic but wrong for the quota endpoint.

2. **Missing `GroupId` query parameter.** The endpoint requires `?GroupId=<group-id>` (e.g. `2024475421608780062`). Without it, MiniMax returns an error. The GroupId is the user's MiniMax organization ID, visible on the platform dashboard. csq must store this as a per-slot configuration value alongside the API key.

3. **Wrong response parser.** csq's `poll_minimax_quota` (line 861-887) expects `data.remaining` / `data.total`. The actual response is:

```json
{
  "model_remains": [
    {
      "model_name": "MiniMax-M*",
      "current_interval_total_count": 30000,
      "current_interval_usage_count": 29850,
      "current_weekly_total_count": 300000,
      "current_weekly_usage_count": 289423,
      "start_time": 1775988000000,
      "end_time": 1776006000000,
      "remains_time": 281019
    }
  ]
}
```

The parser must: (a) iterate `model_remains[]`, (b) find the entry matching the user's configured model (or `MiniMax-M*` for the coding plan), (c) compute usage percentage as `current_interval_usage_count / current_interval_total_count * 100`.

**Fix plan:**

- Change host to `platform.minimax.io` in `poll_minimax_quota`.
- Add `GroupId` to per-slot settings (`config-<N>/settings.json` env block or a new field). User provides it during account setup.
- Rewrite the response parser for the actual `model_remains[]` shape.
- Add a `current_weekly_*` breakdown for the 7-day equivalent display.

## 5.4 Z.AI (RESOLVED — auth barrier)

**Investigated 2026-04-12 via Playwright MCP.** Z.AI has a direct quota API. The problem is authentication.

**Working endpoint (from browser):**

```
GET https://api.z.ai/api/monitor/usage/quota/limit
Authorization: Bearer <JWT_SESSION_TOKEN>
Accept: application/json
```

**Response:**

```json
{
  "code": 200,
  "data": {
    "limits": [
      {
        "type": "TIME_LIMIT",
        "unit": 5,
        "number": 1,
        "usage": 4000,
        "currentValue": 264,
        "remaining": 3736,
        "percentage": 6,
        "nextResetTime": 1778376833998,
        "usageDetails": [
          { "modelCode": "search-prime", "usage": 264 },
          { "modelCode": "web-reader", "usage": 0 }
        ]
      },
      {
        "type": "TOKENS_LIMIT",
        "unit": 3,
        "number": 5,
        "percentage": 11,
        "nextResetTime": 1776007017081
      },
      {
        "type": "TOKENS_LIMIT",
        "unit": 6,
        "number": 1,
        "percentage": 9,
        "nextResetTime": 1776389633997
      }
    ],
    "level": "max"
  }
}
```

Subscription confirmed via `GET api.z.ai/api/biz/subscription/list`: **GLM Coding Max** ($216/quarter, auto-renew).

**Auth barrier:** The JWT session token (`z-ai-open-platform-token-production` in browser localStorage) is NOT the API key csq stores. It's obtained via Z.AI's OAuth login callback flow (`z.ai/login/callback?code=<authcode>&redirect=...`). The API key (stored in csq per-slot `settings.json` as `ANTHROPIC_AUTH_TOKEN`) is for the Anthropic-compatible message endpoint, not for Z.AI's internal billing APIs.

**Options (in order of preference):**

1. **Implement Z.AI OAuth login in csq.** When user configures a Z.AI slot, csq opens a browser to `z.ai/login/callback`, captures the JWT, stores it alongside the API key. Daemon uses the JWT for quota polling. JWT refresh needs investigation (expiry unknown, possibly long-lived).

2. **Ask user to paste JWT from browser.** `csq login --zai-token <jwt>` — one-time manual step. Stored securely in per-slot settings. Simpler than full OAuth but worse UX.

3. **Show dashboard link instead of live data.** Display "View usage on z.ai" in the dashboard Accounts tab for Z.AI slots. No quota numbers, but no auth complexity. The `max_tokens=1` probe stays as a 429 detector (knows when blocked, just can't show percentage).

**Recommendation:** Option 3 for now (ship something), option 1 as follow-up. Z.AI's quota API is well-structured and the JWT auth is standard — implementing it is a bounded task, just not critical-path for the handle-dir model.

## 5.5 Write invariants

Regardless of source (Anthropic or 3P), the daemon usage poller writes to `quota.json`:

- **One writer**: the usage poller task only. Enforced by rule 1 of `account-terminal-separation.md`.
- **Atomic**: temp file + rename with `0o600` permissions.
- **Per-account keyed**: `quota.json.accounts.<N>` structure preserved. See `csq-core/src/quota/state.rs`.
- **`updated_at` timestamp**: every write stamps the current UNIX time as a float seconds since epoch. Freshness checks (e.g. the dashboard staleness badge — future work) read this field.
- **Rate limits data**: for 3P slots that produce `anthropic-ratelimit-*` headers, the poller ALSO stores `rate_limits` on the account record. Anthropic accounts do not populate this field.

## 5.6 Cooldown and backoff (CRITICAL BUG FIX)

On 2026-04-12 the daemon's usage poller stopped firing after the 12:17 UTC tick. Log evidence showed it successfully completed the 4th Anthropic tick and the `tick_3p` call, then went silent. No panic log, no error. The root cause is almost certainly a blocking HTTP call in `tick_3p` that exceeded the 10-second `reqwest` client timeout (or hung on a TLS handshake under certain conditions) and blocked the `await` on `spawn_blocking` indefinitely.

**Mandatory fixes for the refresh + poller supervisor:**

1. **Per-call timeout**: wrap every `tokio::task::spawn_blocking(|| poll_anthropic_usage(...))` and `spawn_blocking(|| poll_3p_usage(...))` result in `tokio::time::timeout(30s, join_handle)`. On timeout, abort the join handle, log `warn!`, and treat as transient failure (enter cooldown).
2. **Supervised main loop**: `run_loop` MUST be spawned under a supervisor that respawns on panic with exponential backoff, logging the panic payload. Currently the task is `tokio::spawn`ed and its panic dies silently.
3. **Health heartbeat**: the main loop emits a DEBUG log every tick ("usage poller tick complete"). The supervisor checks this heartbeat every 60s; if absent for >3× the expected interval, force-restart the poller subsystem.

These fixes live in the implementation scope of the upgrade that lands specs 01-04. They do not require architecture changes, only hardening.

## 5.7 Cross-references

- `specs/04-csq-daemon-architecture.md` section 4.2.2 — usage poller subsystem.
- `rules/account-terminal-separation.md` rules 1, 2, 4 — quota writer and source-of-truth invariants.
- `csq-core/src/daemon/usage_poller.rs` — implementation site.
- Journal `0028-DECISION-account-terminal-separation-python-elimination.md` — utilization-as-percentage discovery.
- Journal `0025-DISCOVERY-per-slot-third-party-provider-bindings.md` — per-slot 3P binding model.

## Revisions

- 2026-04-12 — 1.0.0 — Initial draft. Sections 5.2-5.4 pending Playwright investigation. Section 5.6 documents the 2026-04-12 poller hang and mandates supervisor + per-call timeout fixes.
