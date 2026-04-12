# 0032 DISCOVERY: 3P Quota APIs Live-Verified — Spec Corrections

**Date:** 2026-04-12  
**Status:** VERIFIED  
**Relates to:** specs/05-quota-polling-contracts.md sections 5.3, 5.4; issues #77, #78

## Context

Specs 05 sections 5.3 (MiniMax) and 5.4 (Z.AI) were written from Playwright browser captures. Two assumptions proved wrong when tested with direct `curl` calls using the stored API keys.

## Finding 1: Z.AI API Key Works for Quota

**Spec claimed:** JWT session token required (`z-ai-open-platform-token-production` from browser localStorage). API key insufficient.

**Reality:** `GET https://api.z.ai/api/monitor/usage/quota/limit` with `Authorization: Bearer <API_KEY>` (the same key stored in per-slot `settings.json`) returns 200 with full quota data:

```json
{ "data": { "limits": [
  { "type": "TOKENS_LIMIT", "unit": 3, "percentage": 6, "nextResetTime": ... },
  { "type": "TOKENS_LIMIT", "unit": 6, "percentage": 11, "nextResetTime": ... }
], "level": "max" } }
```

Unit mapping: 3 = 5-hour, 6 = 7-day. `percentage` is already 0-100.

**Why the spec was wrong:** Playwright captured the browser making the call with both cookies AND the Authorization header. The spec attributed auth to the JWT cookie; the header was sufficient alone.

## Finding 2: MiniMax GroupId Is Optional

**Spec claimed:** `?GroupId=<group-id>` is required. Without it, "MiniMax returns an error."

**Reality:** The endpoint works without GroupId. `GET https://platform.minimax.io/v1/api/openplatform/coding_plan/remains` with just `Authorization: Bearer <API_KEY>` returns 200 with all models.

## Finding 3: MiniMax "usage_count" Is Remaining, Not Consumed

The endpoint is `/coding_plan/remains`. Field `current_interval_usage_count` = remaining usable count, NOT consumed count. `used = total - usage_count`.

Live example: `total=30000, usage_count=29957` means 29957 REMAIN, 43 consumed, 0.14% used.

## Impact

- PR #79: MiniMax fix shipped with correct host + parser
- PR #80: Z.AI polling fully implemented (was previously skipped)
- Spec 05 needs revision to correct these three findings
