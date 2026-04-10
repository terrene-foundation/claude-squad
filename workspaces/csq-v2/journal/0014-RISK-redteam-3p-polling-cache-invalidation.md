---
type: RISK
date: 2026-04-11
created_at: 2026-04-11T14:00:00+08:00
author: agent
session_id: session-2026-04-11
session_turn: 45
project: csq-v2
topic: Red team findings from 3P polling and cache invalidation PRs
phase: redteam
tags: [security, 3p-polling, cache-invalidation, crlf, cooldown, discovery]
---

# RISK-0014: Red team round 1 findings — 3P polling + cache invalidation

## Context

Red team validation of PRs #54 (3P usage polling) and #55 (swap cache invalidation) identified three HIGH findings, all fixed in PR #56.

## Findings (all resolved)

### H3 — CRLF injection in Unix socket client (FIXED)

`http_get_unix` and `http_post_unix` interpolated `path_and_query` directly into an HTTP request line via `format!()`. A `\r\n` in the path would inject arbitrary HTTP headers. All current callers were safe (string literals or `AccountNum.get()`), but the function is `pub`.

**Fix:** Replaced `debug_assert!` with runtime `validate_path_and_query()` that rejects `\r` and `\n` characters. Four regression tests.

### HIGH — Cooldown ID collision: Anthropic 901 = Z.AI 901 (FIXED)

`tick` (Anthropic) and `tick_3p` (3P) shared the same cooldown/backoff `HashMap<u16, _>` maps. 3P accounts use synthetic IDs (901 Z.AI, 902 MiniMax) that overlap with valid Anthropic `AccountNum` range (1..999). A 429 on one source could suppress polling for the other.

**Fix:** Separate `cooldowns_3p`/`backoffs_3p` maps in `RunLoopConfig`.

### HIGH — Discovery/polling key path mismatch (FIXED)

`discover_third_party()` checked for top-level `ANTHROPIC_AUTH_TOKEN` in the settings JSON. But `ProviderSettings::get_api_key()` reads from `env.ANTHROPIC_AUTH_TOKEN` (nested). A settings file with keys only in the canonical `env` location would be discovered but never polled (silent skip).

**Fix:** Discovery now checks both top-level and `env.` subobject. Regression test covers the nested-only case.

## Accepted MEDIUMs (tracked, not blocking)

- **M1 — Concurrent read-modify-write on `quota.json`**: Within the daemon, `tick` and `tick_3p` run sequentially. Cross-process races (daemon vs CLI `csq status --update`) could lose a write, but impact is cosmetic (stale percentage, not credential loss).
- **M2 — Hardcoded probe model name**: `PROBE_BODY` uses `claude-sonnet-4-20250514`. If deprecated, 3P polling silently degrades to 400 responses. Track for M9 config-driven model selection.
- **H1 (downgraded to track) — `redact_tokens` gap for 3P keys**: Current 3P polling code never logs error inner strings, so the risk is latent. Future callers of `PollError` variants could leak 3P API keys if they format the inner `String`. Recommend expanding `redact_tokens` regex in a future session.

## Convergence

- Round 1: 3 HIGH findings identified
- Round 1 fix: PR #56 merged, all 3 HIGHs resolved
- Round 2: All fixes verified correct and complete, no new issues
- **Converged: 0 CRITICAL, 0 HIGH, 2 consecutive clean rounds**

## For Discussion

1. Should `AccountNum` reserve the 900+ range for synthetic 3P IDs to prevent future collision at the type level rather than at the map level?
2. If the hardcoded probe model `claude-sonnet-4-20250514` is retired by Anthropic, what's the detection and recovery path? Should the daemon log a warning after N consecutive 400s from 3P polling?
3. Given that the discovery module checks two different JSON paths (top-level and `env.`), should the top-level check be deprecated in favor of always reading from `env.`?
