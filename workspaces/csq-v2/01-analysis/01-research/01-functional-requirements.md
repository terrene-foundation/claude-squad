# csq v2.0 — Functional Requirements

Exhaustive inventory of every feature in v1.x that must be preserved or evolved in v2.0.
Source: function-by-function walkthrough of `csq` (bash), `rotation-engine.py`, `dashboard/`, `statusline-quota.sh`, and `install.sh`.

---

## 1. Account Management

### 1.1 Login (OAuth PKCE)

| ID     | Feature                          | Source                                            | Details                                                                                                                      |
| ------ | -------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| AM-001 | Browser-based OAuth login        | `csq:cmd_login()` L86-195                         | Opens browser via `claude auth login` with isolated `CLAUDE_CONFIG_DIR`                                                      |
| AM-002 | PKCE flow (dashboard)            | `dashboard/oauth.py:OAuthLogin` L50-308           | Full OAuth 2.0 Authorization Code + PKCE. Generates code_verifier/challenge, builds authorize URL, exchanges code for tokens |
| AM-003 | Post-login email capture         | `csq:cmd_login()` L101-104                        | Runs `claude auth status --json` after login to extract email                                                                |
| AM-004 | Credential capture from keychain | `csq:cmd_login()` L109-138                        | macOS: reads from `security find-generic-password` using SHA256(NFC(config_dir))[:8] service name                            |
| AM-005 | Credential capture from file     | `csq:cmd_login()` L140-148                        | Fallback: reads `config-N/.credentials.json` directly                                                                        |
| AM-006 | Canonical credential save        | `csq:cmd_login()` L155-159                        | Saves to `credentials/N.json` with `0o600` permissions                                                                       |
| AM-007 | Config-dir credential mirror     | `csq:cmd_login()` L162-166                        | Copies credentials to `config-N/.credentials.json` so CC reads them at startup                                               |
| AM-008 | Profile save                     | `csq:cmd_login()` L182-194                        | Writes email + method to `profiles.json` under `accounts.N`                                                                  |
| AM-009 | Broker-failure flag clear        | `csq:cmd_login()` L179                            | Removes `credentials/N.broker-failed` on fresh login (the "user fixed it" signal)                                            |
| AM-010 | Account number validation        | `rotation-engine.py:_validate_account()` L684-693 | Validates 1..999, digits only. Prevents path traversal and keychain namespace injection                                      |

### 1.2 Account Discovery

| ID     | Feature                           | Source                                                        | Details                                                                                                                     |
| ------ | --------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| AM-011 | Discover Anthropic OAuth accounts | `dashboard/accounts.py:discover_anthropic_accounts()` L93-154 | Scans `credentials/*.json` for numeric filenames, reads `claudeAiOauth.accessToken`, matches with `profiles.json` for email |
| AM-012 | Discover 3P accounts              | `dashboard/accounts.py:discover_3p_accounts()` L157-207       | Reads `settings-zai.json`, `settings-mm.json` for `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL`                           |
| AM-013 | Load manual accounts              | `dashboard/accounts.py:load_manual_accounts()` L210-246       | Reads `dashboard-accounts.json` for manually added accounts                                                                 |
| AM-014 | Combined discovery with dedup     | `dashboard/accounts.py:discover_all_accounts()` L311-345      | Merges Anthropic + 3P + manual, deduplicates by ID (first wins)                                                             |
| AM-015 | Save manual account               | `dashboard/accounts.py:save_manual_account()` L249-308        | Appends to `dashboard-accounts.json`, atomic write with `0o600`                                                             |

### 1.3 Account Identity Detection

| ID     | Feature                                    | Source                                                   | Details                                                                                    |
| ------ | ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| AM-016 | which_account() — fast path                | `rotation-engine.py:which_account()` L283-334            | Reads `.current-account` from `CLAUDE_CONFIG_DIR`                                          |
| AM-017 | which_account() — config dir name fallback | `rotation-engine.py:which_account()` L306-313            | Extracts N from `config-N` directory name                                                  |
| AM-018 | which_account() — CC auth fallback         | `rotation-engine.py:which_account()` L316-334            | Runs `claude auth status --json` and matches email to profiles                             |
| AM-019 | csq_account_marker()                       | `rotation-engine.py:csq_account_marker()` L552-579       | Reads `.csq-account` — the durable identity marker written by `csq run` and `csq swap`     |
| AM-020 | credentials_file_account()                 | `rotation-engine.py:credentials_file_account()` L486-506 | Matches access token in `.credentials.json` against all `credentials/N.json`               |
| AM-021 | live_credentials_account()                 | `rotation-engine.py:live_credentials_account()` L509-549 | Matches refresh token — race-proof ground truth for "which account CC is actually running" |
| AM-022 | write_csq_account_marker()                 | `rotation-engine.py:write_csq_account_marker()` L582-596 | Atomic write of `.csq-account` marker                                                      |

### 1.4 Swap (In-Place Account Switch)

| ID     | Feature                            | Source                                   | Details                                                                                                                                                                              |
| ------ | ---------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| AM-023 | swap_to() — core swap              | `rotation-engine.py:swap_to()` L840-1014 | Reads cached creds from `credentials/N.json`, writes to `.credentials.json` (atomic), `.csq-account`, `.current-account`. Never calls refresh endpoint (CC handles its own refresh). |
| AM-024 | Swap verification                  | `rotation-engine.py:swap_to()` L971-984  | Reads back `.credentials.json` immediately to verify write succeeded                                                                                                                 |
| AM-025 | Delayed swap verification          | `rotation-engine.py:swap_to()` L995-1013 | Background thread checks at +2s if CC overwrote the swap                                                                                                                             |
| AM-026 | Best-effort keychain write on swap | `rotation-engine.py:swap_to()` L940-943  | Tries `write_keychain()`, never blocks or fails the swap                                                                                                                             |
| AM-027 | Quota cursor preservation on swap  | `rotation-engine.py:swap_to()` L925-931  | Does NOT delete `.quota-cursor` — protects against stale rate-limits corruption                                                                                                      |

---

## 2. Credential Management

### 2.1 OAuth Token Refresh

| ID     | Feature                          | Source                                               | Details                                                                                                                                                    |
| ------ | -------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CR-001 | refresh_token()                  | `rotation-engine.py:refresh_token()` L695-768        | POST to `platform.claude.com/v1/oauth/token` with `grant_type=refresh_token`. Atomic write to canonical. Preserves `subscriptionType` and `rateLimitTier`. |
| CR-002 | Dashboard proactive refresh      | `dashboard/refresher.py:TokenRefresher` L55-455      | Background thread checks every 5 minutes, refreshes when token expires within 30 minutes. 10-minute cooldown after failures.                               |
| CR-003 | Monotonicity guard (dashboard)   | `dashboard/refresher.py:_do_refresh()` L307-327      | Re-reads credentials after HTTP refresh; if `expiresAt` is newer than pre-read, another process won — skips write                                          |
| CR-004 | Manual token refresh (dashboard) | `dashboard/refresher.py:refresh_account()` L105-142  | API endpoint to force refresh a specific account, respects cooldown                                                                                        |
| CR-005 | Token health reporting           | `dashboard/refresher.py:get_token_status()` L144-207 | Returns `is_healthy`, `expires_in_seconds`, `last_refresh`, `error`                                                                                        |

### 2.2 Broker (Centralized Token Refresh)

| ID     | Feature                            | Source                                                                         | Details                                                                                                                                                                                      |
| ------ | ---------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CR-006 | broker_check()                     | `rotation-engine.py:broker_check()` L1702-1795                                 | Per-account try-lock. Reads canonical, checks expiry (2-hour window). Refreshes via `refresh_token()`, fans out to all config dirs. Non-blocking — skips if another terminal holds the lock. |
| CR-007 | Broker recovery from live siblings | `rotation-engine.py:_broker_recover_from_live()` L1634-1699                    | When canonical RT is dead (CC won a race), tries each live sibling's RT. Promotes candidate into canonical, retries refresh. Restores original on total failure.                             |
| CR-008 | Broker failure flag                | `rotation-engine.py:_broker_mark_failed()/_broker_mark_recovered()` L1620-1631 | Touches `credentials/N.broker-failed` on total failure. Surfaces `LOGIN-NEEDED` in statusline. Cleared by successful refresh or `csq login`.                                                 |
| CR-009 | Broker fanout                      | `rotation-engine.py:_fan_out_credentials()` L1583-1606                         | Writes new credentials to every `config-X/.credentials.json` where `.csq-account` marker matches. Atomic per-file. Skip if already in sync.                                                  |
| CR-010 | Config dir scanning                | `rotation-engine.py:_scan_config_dirs_for_account()` L1556-1580                | Scans `config-*` dirs for matching `.csq-account` markers                                                                                                                                    |

### 2.3 Backsync and Pullsync

| ID     | Feature         | Source                                        | Details                                                                                                                                                                                                                                                          |
| ------ | --------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CR-011 | backsync()      | `rotation-engine.py:backsync()` L1366-1503    | Live `.credentials.json` -> canonical `credentials/N.json` when live is newer. Content-match by refresh token (primary) or `.csq-account` marker (fallback for rotated RTs). Per-canonical lock. Monotonicity guard (only writes if `expiresAt` strictly newer). |
| CR-012 | pullsync()      | `rotation-engine.py:pullsync()` L1814-1875    | Canonical `credentials/N.json` -> live `.credentials.json` when canonical is newer. Reads marker for account ID. Only writes if `expiresAt` strictly newer and access tokens differ.                                                                             |
| CR-013 | sync (combined) | `rotation-engine.py:main()` "sync" L1934-1945 | Runs `broker_check()` + `backsync()` + `pullsync()` in sequence. Called from statusline hook.                                                                                                                                                                    |

### 2.4 Keychain Integration

| ID     | Feature                  | Source                                                 | Details                                                                                                            |
| ------ | ------------------------ | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| CR-014 | \_keychain_service()     | `rotation-engine.py:_keychain_service()` L774-783      | Derives service name: `Claude Code-credentials-{sha256(NFC(dir))[:8]}`                                             |
| CR-015 | write_keychain()         | `rotation-engine.py:write_keychain()` L786-812         | macOS only: hex-encodes JSON, writes via `security add-generic-password -U`. 3-second timeout. No-op on non-macOS. |
| CR-016 | write_credentials_file() | `rotation-engine.py:write_credentials_file()` L815-834 | Atomic write to `CLAUDE_CONFIG_DIR/.credentials.json`                                                              |

### 2.5 Atomic File Operations

| ID     | Feature                | Source                                                      | Details                                                                               |
| ------ | ---------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| CR-017 | \_atomic_replace()     | `rotation-engine.py:_atomic_replace()` L195-205             | `os.replace()` with retry (5 attempts, 100ms delay) for Windows file-in-use conflicts |
| CR-018 | \_secure_file()        | `rotation-engine.py:_secure_file()` L186-193                | `chmod 0o600`. No-op on Windows.                                                      |
| CR-019 | File locking — POSIX   | `rotation-engine.py:_lock_file()/_try_lock_file()` L162-183 | `fcntl.flock()` — advisory, whole-file                                                |
| CR-020 | File locking — Windows | `rotation-engine.py:_lock_file()/_try_lock_file()` L125-157 | Named mutex via `kernel32.CreateMutexW`. Non-blocking variant returns None if held.   |

---

## 3. Session Management

### 3.1 Run (Launch Claude Code)

| ID     | Feature                           | Source                             | Details                                                                                                                                                                                                                               |
| ------ | --------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SM-001 | Account auto-resolution           | `csq:cmd_run()` L201-257           | 0 accounts -> vanilla `claude`. 1 account -> uses it. 2+ -> error requiring explicit N.                                                                                                                                               |
| SM-002 | Profile overlay support           | `csq:cmd_run()` L213-226, L428-485 | `--profile`/`-p` flag. Deep-merges `settings-<name>.json` over `settings.json`. Detects if profile provides its own auth (ANTHROPIC_AUTH_TOKEN).                                                                                      |
| SM-003 | Config dir isolation              | `csq:cmd_run()` L259-349           | Symlinks shared artifacts from `~/.claude` (history, sessions, commands, skills, etc.). Isolates: `.credentials.json`, `.current-account`, `.csq-account`, `.live-pid`, `.claude.json`, `accounts`, `settings.json`, `.quota-cursor`. |
| SM-004 | Windows junction support          | `csq:cmd_run()` L340-348           | Uses `mklink /J` for directory junctions (no admin required), falls back to `cp`                                                                                                                                                      |
| SM-005 | Synchronous broker refresh on run | `csq:cmd_run()` L383-385           | Calls `rotation-engine.py broker` before copying canonical. Aborts with clear message if token is dead.                                                                                                                               |
| SM-006 | Credential copy at startup        | `csq:cmd_run()` L392-397           | Atomic copy from `credentials/N.json` to `config-N/.credentials.json`                                                                                                                                                                 |
| SM-007 | Onboarding flag                   | `csq:cmd_run()` L415-425           | Sets `hasCompletedOnboarding=true` in `.claude.json` to skip CC's setup wizard                                                                                                                                                        |
| SM-008 | Settings.json merge               | `csq:cmd_run()` L428-485           | Builds per-terminal `settings.json` from default + optional profile overlay. Supports truncated JSON auto-repair.                                                                                                                     |
| SM-009 | Env var stripping                 | `csq:cmd_run()` L497               | Unsets `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` to prevent conflict with profile auth                                                                                                                                           |
| SM-010 | Pass-through claude args          | `csq:cmd_run()` L499               | Additional args (e.g., `--resume`) passed to `claude` via exec                                                                                                                                                                        |
| SM-011 | Stale .live-pid cleanup           | `csq:cmd_run()` L407               | Removes `.live-pid` from prior CC process so first statusline re-snapshots                                                                                                                                                            |

### 3.2 Live-Account Snapshot

| ID     | Feature                           | Source                                               | Details                                                                                                                                                                                                              |
| ------ | --------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SM-012 | snapshot_account()                | `rotation-engine.py:snapshot_account()` L599-643     | Triggered from statusline on every render. Cheap path: if `.live-pid` process is alive, no-op. Expensive path: walk process tree to find CC PID, read `.csq-account` marker, write `.current-account` + `.live-pid`. |
| SM-013 | \_find_cc_pid() — POSIX           | `rotation-engine.py:_find_cc_pid_posix()` L397-427   | Walks parent process tree via `ps -p PID -o ppid=,command=`. Up to 20 levels.                                                                                                                                        |
| SM-014 | \_find_cc_pid() — Windows         | `rotation-engine.py:_find_cc_pid_windows()` L430-465 | Uses `CreateToolhelp32Snapshot` for single kernel call. Builds PID-to-parent map, walks parent chain.                                                                                                                |
| SM-015 | \_is_pid_alive() — cross-platform | `rotation-engine.py:_is_pid_alive()` L347-368        | POSIX: `os.kill(pid, 0)`. Windows: `OpenProcess + GetExitCodeProcess`.                                                                                                                                               |

---

## 4. Usage Monitoring

### 4.1 Quota Tracking

| ID     | Feature                | Source                                           | Details                                                                                                                                                                           |
| ------ | ---------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| UM-001 | update_quota()         | `rotation-engine.py:update_quota()` L1099-1203   | Parses statusline JSON, extracts `rate_limits`. Uses live_credentials_account() for ground truth. Payload-hash cursor prevents stale data after swap. File locking on quota.json. |
| UM-002 | Quota state management | `rotation-engine.py:load_state()` L251-263       | Loads `quota.json`, auto-clears expired windows based on `resets_at` timestamps                                                                                                   |
| UM-003 | show_status()          | `rotation-engine.py:show_status()` L1218-1245    | Displays all accounts: active marker, email, 5h/7d usage percentages, reset times. Icons: `bullet` (<80%), `half` (80-99%), `circle` (100%).                                      |
| UM-004 | statusline_str()       | `rotation-engine.py:statusline_str()` L1248-1306 | Compact statusline: `#N:user 5h:X% 7d:Y%`. Stuck-swap warning (`#N!:user`). Broker-failure warning (`LOGIN-NEEDED` prefix). Self-healing stale broker flags.                      |
| UM-005 | suggest()              | `rotation-engine.py:suggest()` L1020-1045        | JSON output: best account to switch to. Excludes current. Returns `exhausted: true` if all at 100%.                                                                               |
| UM-006 | pick_best()            | `rotation-engine.py:pick_best()` L649-678        | Selects account with lowest 5h usage. If all exhausted, picks earliest reset time.                                                                                                |

### 4.2 Usage Polling (Dashboard)

| ID     | Feature                      | Source                                                       | Details                                                                                                                                                                               |
| ------ | ---------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| UM-007 | Anthropic usage polling      | `dashboard/poller.py:poll_anthropic_usage()` L48-101         | GET `/api/oauth/usage` with Bearer token + `Anthropic-Beta: oauth-2025-04-20` header                                                                                                  |
| UM-008 | 3P usage polling             | `dashboard/poller.py:poll_3p_usage()` L104-164               | POST `/v1/messages` with `max_tokens=1` to extract rate-limit headers from response                                                                                                   |
| UM-009 | Rate-limit header extraction | `dashboard/poller.py:_extract_rate_limit_headers()` L167-197 | Parses `anthropic-ratelimit-*` headers: requests limit/remaining, tokens limit/remaining, input/output limits                                                                         |
| UM-010 | Background poller            | `dashboard/poller.py:UsagePoller` L200-369                   | Background thread. Anthropic: 5-min interval. 3P: 15-min interval. Staggered initial polls (5s between accounts). Exponential backoff on 429 (doubles, max 8x). 401 marks as expired. |
| UM-011 | Force refresh                | `dashboard/poller.py:UsagePoller.force_refresh()` L250-268   | Respects rate-limit intervals; returns "skipped" for too-recent accounts                                                                                                              |
| UM-012 | In-memory cache with TTL     | `dashboard/cache.py:UsageCache` L16-93                       | Thread-safe dict, per-entry timestamps, configurable max_age_seconds (default 10 min)                                                                                                 |

### 4.3 Statusline Hook

| ID     | Feature                 | Source                                        | Details                                                                                                               |
| ------ | ----------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------- | ----- | ----- | ------- | ----------- |
| UM-013 | Statusline script       | `statusline-quota.sh` L1-148                  | Runs on every CC render. Extracts workspace, model, context window, cost from CC's JSON.                              |
| UM-014 | Synchronous snapshot    | `statusline-quota.sh` L40                     | Runs `rotation-engine.py snapshot` BEFORE update (attributes data to correct account)                                 |
| UM-015 | Background sync         | `statusline-quota.sh` L52                     | Runs `rotation-engine.py sync` in background (broker + backsync + pullsync)                                           |
| UM-016 | Background quota update | `statusline-quota.sh` L55                     | Pipes CC's JSON to `rotation-engine.py update` in background                                                          |
| UM-017 | Statusline format       | `statusline-quota.sh` L105-147                | Format: `csq #N:user 5h:X% 7d:Y%                                                                                      | ctx:45k 62% | $1.23 | model | project | git:branch` |
| UM-018 | Context window tracking | `statusline-quota.sh` L26-31                  | Extracts `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `used_percentage` |
| UM-019 | Session cost tracking   | `statusline-quota.sh` L34                     | Extracts `cost.total_cost_usd`                                                                                        |
| UM-020 | Git status              | `statusline-quota.sh:get_git_status()` L75-84 | Shows `git:branch` with dirty indicator                                                                               |

---

## 5. Provider Support

### 5.1 Provider Profiles

| ID     | Feature                       | Source                                | Details                                                                                                                                                                  |
| ------ | ----------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PS-001 | setkey — API key management   | `csq:cmd_setkey()` L523-752           | Sets API key for provider profile. Supports: `claude` (ANTHROPIC_API_KEY), `mm` (ANTHROPIC_AUTH_TOKEN), `zai` (ANTHROPIC_AUTH_TOKEN), `ollama` (keyless).                |
| PS-002 | Key validation                | `csq:cmd_setkey()` L700-732           | For bearer-token providers: sends `max_tokens=1` test request via `curl`. Reports HTTP status.                                                                           |
| PS-003 | Stdin key reading             | `csq:cmd_setkey()` L538-547           | Reads key from stdin if not passed as arg (keeps key out of shell history). Strips `\r` from Windows clipboard.                                                          |
| PS-004 | Profile overlay creation      | `csq:cmd_setkey()` L644-748           | Creates/updates `settings-<provider>.json`. Preserves existing fields. Seeds with skeleton on first use. Auto-repairs truncated JSON.                                    |
| PS-005 | System prompt primers         | `csq:cmd_setkey()` L563-564, L683-689 | Non-Claude models get `systemPromptFile` (prepend) and `appendSystemPromptFile` (append) for COC compliance. Uses `3p-model-primer.md` and `3p-model-primer-prepend.md`. |
| PS-006 | listkeys                      | `csq:cmd_listkeys()` L802-835         | Shows configured profiles with masked keys: profile name, key status, fingerprint (first 8 + last 6 chars), file path                                                    |
| PS-007 | rmkey                         | `csq:cmd_rmkey()` L837-846            | Removes a provider profile file                                                                                                                                          |
| PS-008 | Profile-aware credential skip | `csq:cmd_run()` L265-299              | If profile provides `ANTHROPIC_AUTH_TOKEN` or `ANTHROPIC_API_KEY` in its env, OAuth credentials are not required for that run                                            |

### 5.2 Provider Configurations

| ID     | Feature           | Source                      | Details                                                                                                                                 |
| ------ | ----------------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| PS-009 | Claude direct API | `csq:cmd_setkey()` L570-575 | `ANTHROPIC_API_KEY` env var. x-api-key auth.                                                                                            |
| PS-010 | MiniMax M2.7      | `csq:cmd_setkey()` L579-595 | `api.minimax.io/anthropic`, bearer auth, all model slots set to `MiniMax-M2.7-highspeed`, 50-min timeout, disable non-essential traffic |
| PS-011 | Z.AI GLM          | `csq:cmd_setkey()` L597-615 | `api.z.ai/api/anthropic`, bearer auth, all model slots set to `glm-5.1`                                                                 |
| PS-012 | Ollama            | `csq:cmd_setkey()` L617-637 | `localhost:11434`, keyless, `ANTHROPIC_AUTH_TOKEN=ollama`, all model slots set to `qwen3:latest`                                        |

---

## 6. Model Management

| ID     | Feature                            | Source                      | Details                                                                                                          |
| ------ | ---------------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| MM-001 | List all profiles + models         | `csq:cmd_models()` L848-920 | Shows profile name, current model, update status (latest/update available). Uses `model-catalog.json`.           |
| MM-002 | List available models for provider | `csq:cmd_models()` L922-956 | From catalog (zai, mm) or from `ollama list` (ollama). Shows active/latest tags.                                 |
| MM-003 | Switch model                       | `csq:cmd_models()` L958-993 | Updates all 5 ANTHROPIC\_\*\_MODEL env vars in the profile. Validates against catalog/ollama list. Atomic write. |
| MM-004 | Model catalog                      | `model-catalog.json`        | JSON file with per-provider: `models` array, `latest`, `default`. Downloaded by `csq update`.                    |
| MM-005 | Ollama model detection             | `csq:cmd_models()` L884-892 | Runs `ollama list` to discover installed models                                                                  |

---

## 7. CLI Interface

### 7.1 Commands

| ID     | Command                               | Source                  | Details                                         |
| ------ | ------------------------------------- | ----------------------- | ----------------------------------------------- |
| CL-001 | `csq` (no args)                       | `csq:main()` L997-1001  | Defaults to `csq run` (auto-resolves account)   |
| CL-002 | `csq <N>`                             | `csq:main()` L1003-1005 | Shorthand for `csq run N`                       |
| CL-003 | `csq run [N] [-p <prof>] [args]`      | `csq:cmd_run()`         | Start CC on account N with optional profile     |
| CL-004 | `csq login <N>`                       | `csq:cmd_login()`       | Save current session as slot N                  |
| CL-005 | `csq swap <N>`                        | `csq:main()` L1027-1031 | Swap THIS terminal (requires CLAUDE_CONFIG_DIR) |
| CL-006 | `csq status` / `csq ls` / `csq quota` | `csq:main()` L1020-1021 | Show all accounts + quota                       |
| CL-007 | `csq suggest`                         | `csq:cmd_suggest()`     | Show best account to switch to                  |
| CL-008 | `csq cleanup`                         | `csq:main()` L1033-1034 | Remove stale PID cache files                    |
| CL-009 | `csq setkey <prov> [key]`             | `csq:cmd_setkey()`      | Set API key for provider                        |
| CL-010 | `csq listkeys` / `csq keys`           | `csq:cmd_listkeys()`    | Show configured provider profiles               |
| CL-011 | `csq rmkey <prov>`                    | `csq:cmd_rmkey()`       | Remove provider profile                         |
| CL-012 | `csq models [prov] [model]`           | `csq:cmd_models()`      | List/switch models                              |
| CL-013 | `csq update` / `csq upgrade`          | `csq:cmd_update()`      | Manual update from GitHub                       |
| CL-014 | `csq help` / `-h` / `--help`          | `csq:main()` L1054-1108 | Help text                                       |

### 7.2 Internal Commands (Rotation Engine)

| ID     | Command                                    | Source                                  | Details                                              |
| ------ | ------------------------------------------ | --------------------------------------- | ---------------------------------------------------- |
| CL-015 | `rotation-engine.py status`                | `rotation-engine.py:show_status()`      | Full status display                                  |
| CL-016 | `rotation-engine.py update`                | `rotation-engine.py:update_quota()`     | Quota update from stdin JSON                         |
| CL-017 | `rotation-engine.py swap <N>`              | `rotation-engine.py:swap_to()`          | Account swap                                         |
| CL-018 | `rotation-engine.py auto-rotate [--force]` | `rotation-engine.py:auto_rotate()`      | Auto-rotate (disabled, callable)                     |
| CL-019 | `rotation-engine.py suggest`               | `rotation-engine.py:suggest()`          | JSON suggestion                                      |
| CL-020 | `rotation-engine.py statusline`            | `rotation-engine.py:statusline_str()`   | Compact statusline string                            |
| CL-021 | `rotation-engine.py check`                 | `rotation-engine.py:main()`             | JSON: should this terminal rotate?                   |
| CL-022 | `rotation-engine.py init-keychain <N>`     | `rotation-engine.py:init_keychain()`    | Write creds to keychain                              |
| CL-023 | `rotation-engine.py snapshot`              | `rotation-engine.py:snapshot_account()` | Refresh account identity on CC restart               |
| CL-024 | `rotation-engine.py backsync`              | `rotation-engine.py:backsync()`         | Live -> canonical sync                               |
| CL-025 | `rotation-engine.py pullsync`              | `rotation-engine.py:pullsync()`         | Canonical -> live sync                               |
| CL-026 | `rotation-engine.py broker`                | `rotation-engine.py:broker_check()`     | Synchronous broker refresh (exit 2 on total failure) |
| CL-027 | `rotation-engine.py sync`                  | combined                                | broker + backsync + pullsync                         |
| CL-028 | `rotation-engine.py email <N>`             | `rotation-engine.py:get_email()`        | Print email for account N                            |
| CL-029 | `rotation-engine.py cleanup`               | `rotation-engine.py:cleanup()`          | Remove stale PID cache files                         |
| CL-030 | `rotation-engine.py python-cmd`            | `rotation-engine.py:_python_cmd()`      | Print resolved Python 3 command                      |

---

## 8. Desktop App / Dashboard

### 8.1 HTTP Server

| ID     | Feature                      | Source                                                      | Details                                                              |
| ------ | ---------------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------- |
| DA-001 | Static file serving          | `dashboard/server.py:_serve_static()` L334-360              | Serves `static/` directory. Path-traversal sanitization. MIME types. |
| DA-002 | GET /api/accounts            | `dashboard/server.py:_handle_api_accounts()` L107-132       | Lists all accounts with usage, timestamps, token health              |
| DA-003 | GET /api/account/{id}/usage  | `dashboard/server.py:_handle_api_account_detail()` L134-151 | Detailed usage for one account                                       |
| DA-004 | GET /api/refresh             | `dashboard/server.py:_handle_api_refresh()` L153-165        | Force refresh all accounts (respects rate limits)                    |
| DA-005 | GET /api/tokens              | `dashboard/server.py:_handle_api_tokens()` L217-240         | Token health for all accounts                                        |
| DA-006 | GET /api/login/{N}           | `dashboard/server.py:_handle_api_login()` L261-292          | Start OAuth login flow                                               |
| DA-007 | GET /oauth/callback          | `dashboard/server.py:_handle_oauth_callback()` L293-330     | OAuth redirect handler. Returns success HTML page.                   |
| DA-008 | POST /api/accounts           | `dashboard/server.py:_handle_post_account()` L167-215       | Add manual account (label, token, provider, base_url)                |
| DA-009 | POST /api/refresh-token/{id} | `dashboard/server.py:_handle_api_refresh_token()` L242-259  | Manually trigger token refresh                                       |

### 8.2 Background Services

| ID     | Feature              | Source                                  | Details                                       |
| ------ | -------------------- | --------------------------------------- | --------------------------------------------- |
| DA-010 | Usage polling daemon | `dashboard/poller.py:UsagePoller`       | See UM-010. Started by `create_server()`.     |
| DA-011 | Token refresh daemon | `dashboard/refresher.py:TokenRefresher` | See CR-002. Started by `create_server()`.     |
| DA-012 | OAuth handler        | `dashboard/oauth.py:OAuthLogin`         | See AM-002. Initialized by `create_server()`. |

### 8.3 Dashboard UI

| ID     | Feature                 | Source                          | Details                                                                     |
| ------ | ----------------------- | ------------------------------- | --------------------------------------------------------------------------- |
| DA-013 | Account list view       | `dashboard/static/index.html`   | Shows all accounts: provider, email, usage bars, token health, last updated |
| DA-014 | Login from browser      | `dashboard/static/dashboard.js` | "Add Account" button triggers OAuth flow                                    |
| DA-015 | Manual refresh button   | `dashboard/static/dashboard.js` | Force-refresh usage data                                                    |
| DA-016 | Token health indicators | `dashboard/static/dashboard.js` | Visual indicators for token expiry                                          |

---

## 9. Installation and Updates

| ID     | Feature                  | Source                         | Details                                                                                                                                   |
| ------ | ------------------------ | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| IN-001 | Installer script         | `install.sh` L1-205            | Platform detection (macOS/Linux/WSL/Git Bash). Python detection. Creates directories with 0o700. Installs files. Creates config dirs 1-7. |
| IN-002 | Settings.json patching   | `install.sh` L151-182          | Sets statusline command. Removes dead auto-rotate hook.                                                                                   |
| IN-003 | Migration cleanup        | `install.sh` L125-127          | Removes stale `statusline-command.sh` and obsolete `/rotate` command from pre-2.x installs                                                |
| IN-004 | Auto-update (background) | `csq:_auto_update_bg()` L54-84 | Runs on every `csq run`. Downloads remote csq with 3s timeout, compares, updates all files if changed.                                    |
| IN-005 | Manual update            | `csq:cmd_update()` L755-799    | Downloads all files from GitHub main. Reports if already up to date.                                                                      |
| IN-006 | Curl-pipe installer      | `install.sh` L8                | `curl -sSL .../install.sh                                                                                                                 | bash` for remote install |
| IN-007 | Local install mode       | `install.sh` L94-107           | Detects if running from repo clone; copies local files instead of curl                                                                    |

---

## 10. Auto-Rotate (Disabled but Preserved)

| ID     | Feature             | Source                                        | Details                                                                                                                                    |
| ------ | ------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| AR-001 | auto_rotate()       | `rotation-engine.py:auto_rotate()` L1051-1093 | Still callable via CLI. Marks current as exhausted (with lock), picks best, swaps. Not invoked by any running code path. See journal/0010. |
| AR-002 | auto-rotate-hook.sh | `install.sh` L114-120                         | Written as `exit 0` no-op. Kept for wiring continuity.                                                                                     |
