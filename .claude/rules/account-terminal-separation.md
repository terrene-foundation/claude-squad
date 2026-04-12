# Account/Terminal Separation

Applies to ALL code that reads, writes, or displays account credentials, quota data, or usage information.

**Authoritative specs:** `specs/01-cc-credential-architecture.md` (how CC handles credentials), `specs/02-csq-handle-dir-model.md` (csq on-disk layout, handle-dir lifecycle, swap semantics). If this rule and a spec disagree, the spec wins and this rule must be corrected.

## The Two Entities

### Accounts

An Account is an independently authenticated Anthropic identity (email + OAuth tokens). Each account:

- Has a permanent canonical directory `config-<N>/` that lives forever (spec 02, INV-01).
- Has its own credentials (`config-<N>/.credentials.json` and mirror at `credentials/<N>.json`).
- Has its own usage quota (polled by the daemon from Anthropic's `/api/oauth/usage`).
- Auto-refreshes its own tokens (daemon refresher, 5-min cycle).
- Is the SOLE source of truth for its own quota data.

Analogy: an Account is like a Chrome profile logged into `claude.ai/settings/usage`. It independently shows its own usage, refreshes its own session, and never asks any terminal what its usage is.

### Terminals (Sessions)

A Terminal is a CC instance (`claude` process) running with `CLAUDE_CONFIG_DIR` pointing at a handle dir `term-<pid>/`. Each terminal:

- Owns one ephemeral handle dir under `accounts/term-<pid>/` (spec 02, INV-02).
- Reads credentials through a symlink: `term-<pid>/.credentials.json` → `config-<current-account>/.credentials.json`.
- Can swap to any account (`csq swap N`) by atomically repointing its handle dir symlinks.
- Displays the bound account's quota data (read-only).
- NEVER writes quota data.
- NEVER determines which account it "belongs to" for quota purposes — reads `.csq-account` marker via its symlink.

Multiple terminals may bind to the same account. Each has its own handle dir. `csq swap` in one terminal repoints that one handle dir; sibling terminals are physically unaffected because their symlinks still resolve to the old `config-<current>`. Account quota reflects total usage across every handle dir currently symlinked to that account.

## MUST Rules

### 1. Only the Daemon Writes Quota Data

Quota data in `quota.json` MUST only be written by the daemon's usage poller (`daemon/usage_poller.rs`), which polls Anthropic's `/api/oauth/usage` endpoint directly with each account's access token.

```
DO:  daemon polls /api/oauth/usage with account 2's token -> writes to quota.json[2]
DO NOT: terminal reads CC's rate_limits JSON -> writes to quota.json[guessed_account]
```

**Why:** Terminal-to-account attribution is unreliable (marker mismatch, credential contamination, orphaned sessions). Polling Anthropic directly with the account's own token is unforgeable — the response IS that account's usage.

### 2. Terminals Read Quota, Never Write It

The statusline command (`csq statusline`) MUST only READ from `quota.json` and display the result. It MUST NOT call `state::update_quota()` or any function that writes quota data.

```
DO:  csq statusline -> read quota.json -> format -> print
DO NOT: csq statusline -> parse CC JSON -> update_quota() -> print
```

**Why:** CC's per-terminal `rate_limits` JSON is a terminal-scoped snapshot, not an account-scoped measurement. Attributing it to an account requires solving the "which account is this terminal running on?" problem, which has 3+ failure modes (contamination, orphaned credentials, marker drift).

### 3. Accounts Auto-Refresh After Login

After a user authenticates an account (`csq login N`), the daemon MUST automatically:

1. Start refreshing that account's tokens before they expire.
2. Start polling that account's usage from Anthropic.
3. Write refreshed credentials to `config-<N>/.credentials.json` once.

No per-handle-dir fanout is needed. Every handle dir whose symlinks resolve to `config-<N>` automatically sees the new credentials via the symlink on its next `fs.stat` — see spec 01 section 1.4 (CC's mtime reload).

```
DO:  daemon writes config-N/.credentials.json once, symlinks in handle dirs resolve to new file
DO NOT: daemon iterates all term-* dirs and writes per-handle copies (the old fanout model)
```

**Why:** Accounts are independent entities. Once authenticated, they maintain themselves. Handle dirs are views onto account state, not copies — the symlink layer handles fanout for free.

### 4. Account Quota Comes from Anthropic, Not CC

The usage poller reads `utilization` from Anthropic's `/api/oauth/usage` endpoint directly as a percentage (0-100, NOT 0-1 — journal 0028 Discovery). This is the ONLY acceptable source for account-level quota percentages.

```
DO:  Anthropic returns {"utilization": 42.0} -> store used_percentage: 42.0
DO NOT: CC statusline reports {"used_percentage": 2400} -> store used_percentage: 2400.0
```

**Why:** CC's `rate_limits.used_percentage` reflects a single terminal's view and can report values >100% (throttled but not blocked). Anthropic's usage API returns the canonical account-level utilization.

## MUST NOT Rules

### 1. No Terminal-to-Account Attribution for Quota

MUST NOT attempt to determine which account a terminal is running on for the purpose of writing quota data. Functions like `live_credentials_account()` or marker-based attribution MUST NOT be used in quota write paths.

```
DO:  daemon polls Anthropic with account N's token, writes quota.json[N]
DO NOT: terminal reads CC statusline JSON, guesses which account it's on, writes quota.json
```

**Why:** This attribution problem has been the root cause of every quota corruption bug: 1200% on account 2 (marker fallback), cross-contamination (shared refresh tokens), orphaned sessions (fanout miss).

### 2. No statusline JSON in Quota Pipeline

CC's statusline JSON (`rate_limits` field) MUST NOT feed into `quota.json`. The statusline JSON is useful for terminal-local display (e.g., showing usage in the terminal itself) but MUST NOT be persisted or attributed to an account.

```
DO:  csq statusline reads quota.json[account] and formats it
DO NOT: csq statusline reads CC's rate_limits and writes it to quota.json
```

**Why:** The statusline JSON belongs to the terminal, not the account. Persisting it requires solving attribution, which is the problem that caused 6 contamination issues and 2400% phantom usage.

### 3. Identity Derivation Uses Marker, Not Directory Name

The `.csq-account` marker is the SOLE authority for "which account is this session using." In the handle-dir model, a handle dir's `.csq-account` is a symlink resolving to `config-<current-account>/.csq-account`, so reading the marker always returns the current account number correctly.

```
DO:  let account = markers::read_csq_account(&handle_dir).map(|n| n.get())
DO NOT: let account = extract_account_id_from_dir_name(&handle_dir)
DO NOT: let account = extract_account_id_from_dir_name(&config_dir)  // config-N is permanent, so this would work, but code should still read the marker for symmetry
```

**Why:** Handle dir names (`term-<pid>`) contain a PID, not an account number. Config dir names (`config-<N>`) contain account numbers under the new handle-dir model (spec 02, INV-01), but code that reads the marker works correctly in both legacy and new layouts and is uniform across the codebase.

**How to apply:** Any code needing the account number for a directory MUST read `.csq-account`. Falling back to parsing the dir name is UNACCEPTABLE — it silently breaks for handle dirs.

### 4. Credential Writes Preserve Subscription Metadata

`subscription_type` and `rate_limit_tier` are NOT returned by Anthropic's OAuth token endpoint. CC backfills them into the live credentials on first API call. A fresh OAuth response therefore has `None` for both fields.

Any code that writes credentials into `config-<N>/.credentials.json` (daemon refresh, `csq login`) MUST check for missing subscription metadata and preserve the existing value from the current file.

```
DO:  if new_tokens.subscription_type.is_none() { preserve from existing file }
DO NOT: blindly overwrite with subscription_type: None (strips Max tier → CC falls back to Sonnet)
```

**Why:** Overwriting with `subscription_type: None` causes CC to lose its Max tier and default to Sonnet. The user sees the wrong model with no error message. This bug affected all terminals swapped to account 2 in the 2026-04-12 session before the handle-dir model fixed the write path.

**Guard location:** The daemon refresh path in `csq-core/src/daemon/refresher.rs` (and wherever else credentials are written into `config-<N>/.credentials.json`). The old guard in `rotation::swap_to` is obsolete — csq swap does NOT write credentials in the handle-dir model, it repoints symlinks (spec 02, INV-04).

## External Dependencies

### GrowthBook Feature Flags

CC caches server-side A/B test flags from Anthropic's GrowthBook service in `.claude.json` under `cachedGrowthBookFeatures`. The flag `tengu_auto_mode_config` can override model selection silently, regardless of subscription tier.

**Diagnostic:** When a user reports "wrong model" and credentials/subscription look correct, check `.claude.json` for `cachedGrowthBookFeatures.tengu_auto_mode_config`. If it contains `{"enabled": "opt-in", "model": "claude-sonnet-..."}`, that's the cause — not our code.

**Why this matters for csq:** This failure mode is indistinguishable from subscription contamination at the symptom level. Before checking credentials, diff `.claude.json` GrowthBook caches between a working and broken terminal's underlying config dir.

## Retracted Rules

**Rule 7 (Stale Session Detection After Swap) — RETRACTED 2026-04-12.**

The previous rule claimed CC caches credentials at startup and requires a restart after swap, and codified a `needs_restart` badge driven by `.csq-account` marker mtime vs process `started_at`. This was empirically false: CC re-stats `.credentials.json` before every API call path (spec 01 section 1.4; `src/utils/auth.ts:1313-1336` and `:1453`). Swap is in-flight by design.

The handle-dir model replaces the stale-detection heuristic with direct per-terminal independence: each handle dir has its own symlinks, and swap repoints only the affected dir. There is no longer a cross-terminal cache-coherency problem to detect.

Retraction reference: journal 0031. Code artifacts to delete are listed in that journal.

## Cross-References

- `specs/01-cc-credential-architecture.md` — how CC handles credentials (AUTHORITATIVE, cites CC source)
- `specs/02-csq-handle-dir-model.md` — csq on-disk layout, handle-dir lifecycle, swap semantics (AUTHORITATIVE)
- `csq-core/src/daemon/usage_poller.rs` — the ONLY quota writer
- `csq-core/src/daemon/refresher.rs` — account auto-refresh, subscription guard lives here
- `csq-cli/src/commands/statusline.rs` — terminal display (read-only)
- Journal 0029 — earlier findings; Finding 4 is retracted, see Finding 4 banner
- Journal 0031 — retraction + handle-dir adoption
