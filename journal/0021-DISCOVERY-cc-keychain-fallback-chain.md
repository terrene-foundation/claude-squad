---
type: DISCOVERY
title: CC's credential resolution order — keychain fallback does NOT cycle accounts
date: 2026-04-09
---

## Context

Account 7 hit 7d:100% quota. User reported terminal was "still consuming from somewhere else" and believed CC auto-switched accounts via keychain.

## Finding

CC's credential resolution is strictly:

1. Keychain entry for config dir hash (`Claude Code-credentials-{sha256(dir)[:8]}`)
2. Fallback to plaintext `.credentials.json`
3. On 401: invalidate cache, re-read, force refresh
4. On 429: retry up to 10 times with exponential backoff (indefinite in persistent mode)

**CC does NOT enumerate or cycle through keychain entries.** It only reads from ONE entry (matching its config dir). The "still working" behavior was CC's aggressive retry logic (up to 10 retries with backoff) punching through rate limits occasionally. The reset date confirmed it was always account 7.

## Source

Extracted from CC binary v2.1.96 (`strings` analysis) and CC source in `~/repos/contrib/claude-code`:

- `src/utils/secureStorage/macOsKeychainStorage.ts` — keychain I/O
- `src/utils/secureStorage/fallbackStorage.ts` — fallback chain (keychain → plaintext, NOT keychain → keychain)
- `src/services/api/withRetry.ts` — 10 retries, exponential backoff, persistent mode retries indefinitely

## Implication

The broker/csq credential system is accurate — no phantom account switching occurs. CC's retry behavior masks rate limits temporarily but eventually hits the hard wall.
