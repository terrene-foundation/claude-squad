# csq v2.0 — Scope Matrix

Every v1.x function mapped to its v2.0 Rust equivalent: source location, target module, complexity, and test strategy.

Complexity key:

- **Trivial**: Direct translation, no algorithmic complexity, <30 lines of Rust
- **Moderate**: Non-trivial logic, platform conditionals, or concurrency, 30-150 lines
- **Complex**: Race conditions, multi-step algorithms, recovery paths, or cross-platform FFI, 150+ lines

---

## 1. Platform Abstraction Layer

These are foundational primitives that every other module depends on. Build first.

| #    | v1.x Function                      | Source File                   | Lines | v2.0 Module               | Complexity | Test Strategy                                                                               |
| ---- | ---------------------------------- | ----------------------------- | ----- | ------------------------- | ---------- | ------------------------------------------------------------------------------------------- |
| 1.1  | `_detect_platform()`               | `csq` L34-46                  | 13    | `src/platform/mod.rs`     | Trivial    | Unit: `cfg(target_os)` compile-time. No runtime test needed.                                |
| 1.2  | `IS_WINDOWS / IS_MACOS / IS_LINUX` | `rotation-engine.py` L48-50   | 3     | `src/platform/mod.rs`     | Trivial    | Compile-time `cfg` constants.                                                               |
| 1.3  | `_find_python()`                   | `csq` L20-30                  | 11    | REMOVED                   | N/A        | v2.0 has no Python dependency.                                                              |
| 1.4  | `_python_cmd()`                    | `rotation-engine.py` L208-221 | 14    | REMOVED                   | N/A        | v2.0 has no Python dependency.                                                              |
| 1.5  | `_secure_file()`                   | `rotation-engine.py` L186-193 | 8     | `src/platform/fs.rs`      | Trivial    | Unit: verify permissions on Unix. No-op assertion on Windows.                               |
| 1.6  | `_atomic_replace()`                | `rotation-engine.py` L195-205 | 11    | `src/platform/fs.rs`      | Moderate   | Unit: write + rename. Integration: concurrent writers. Windows retry test with locked file. |
| 1.7  | `_lock_file()` POSIX               | `rotation-engine.py` L162-165 | 4     | `src/platform/lock.rs`    | Moderate   | Integration: two processes, one blocks.                                                     |
| 1.8  | `_try_lock_file()` POSIX           | `rotation-engine.py` L167-175 | 9     | `src/platform/lock.rs`    | Moderate   | Integration: try-lock returns None when held by another process.                            |
| 1.9  | `_lock_file()` Windows             | `rotation-engine.py` L125-138 | 14    | `src/platform/lock.rs`    | Complex    | Integration: Windows named mutex. Requires Windows CI runner.                               |
| 1.10 | `_try_lock_file()` Windows         | `rotation-engine.py` L140-152 | 13    | `src/platform/lock.rs`    | Complex    | Integration: non-blocking mutex try.                                                        |
| 1.11 | `_unlock_file()` POSIX + Windows   | `rotation-engine.py` L154-183 | 12    | `src/platform/lock.rs`    | Trivial    | Tested as part of lock/unlock cycles in 1.7-1.10.                                           |
| 1.12 | `_is_pid_alive()` POSIX            | `rotation-engine.py` L347-368 | 22    | `src/platform/process.rs` | Moderate   | Unit: check own PID (alive), check PID 99999999 (dead).                                     |
| 1.13 | `_is_pid_alive()` Windows          | `rotation-engine.py` L349-359 | 11    | `src/platform/process.rs` | Complex    | Unit: same as POSIX but via `OpenProcess + GetExitCodeProcess`.                             |
| 1.14 | `_find_cc_pid_posix()`             | `rotation-engine.py` L397-427 | 31    | `src/platform/process.rs` | Complex    | Integration: spawn a mock process tree, verify correct PID found.                           |
| 1.15 | `_find_cc_pid_windows()`           | `rotation-engine.py` L430-465 | 36    | `src/platform/process.rs` | Complex    | Integration: `CreateToolhelp32Snapshot` walk. Windows CI only.                              |
| 1.16 | `_is_cc_command()`                 | `rotation-engine.py` L371-381 | 11    | `src/platform/process.rs` | Trivial    | Unit: positive/negative command strings.                                                    |
| 1.17 | Win32 ctypes signatures            | `rotation-engine.py` L59-116  | 58    | `src/platform/win32.rs`   | Complex    | Compile-time only (Rust FFI declarations). Runtime tested by 1.9-1.15.                      |

---

## 2. Credential Management

| #    | v1.x Function                      | Source File                                                       | Lines | v2.0 Module                   | Complexity | Test Strategy                                                                                                           |
| ---- | ---------------------------------- | ----------------------------------------------------------------- | ----- | ----------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------- |
| 2.1  | `_load()`                          | `rotation-engine.py` L236-240                                     | 5     | `src/credentials/store.rs`    | Trivial    | Unit: valid JSON, missing file, corrupt file.                                                                           |
| 2.2  | `_save()`                          | `rotation-engine.py` L243-248                                     | 6     | `src/credentials/store.rs`    | Trivial    | Unit: write + read back. Verify permissions.                                                                            |
| 2.3  | `_validate_account()`              | `rotation-engine.py` L684-693                                     | 10    | `src/credentials/validate.rs` | Trivial    | Unit: valid (1, 7, 999), invalid (0, -1, "abc", "../etc").                                                              |
| 2.4  | `refresh_token()`                  | `rotation-engine.py` L695-768                                     | 74    | `src/credentials/oauth.rs`    | Complex    | Unit: mock HTTP server, verify token exchange. Integration: parity with v1.x against live endpoint (manual).            |
| 2.5  | `_keychain_service()`              | `rotation-engine.py` L774-783                                     | 10    | `src/credentials/keychain.rs` | Moderate   | Unit: verify SHA256(NFC(path))[:8] matches v1.x output for known paths. Critical: must be identical to CC's derivation. |
| 2.6  | `write_keychain()`                 | `rotation-engine.py` L786-812                                     | 27    | `src/credentials/keychain.rs` | Moderate   | Integration: write + read back on macOS. Mock on other platforms.                                                       |
| 2.7  | `write_credentials_file()`         | `rotation-engine.py` L815-834                                     | 20    | `src/credentials/store.rs`    | Moderate   | Unit: atomic write to config dir. Verify temp file cleanup.                                                             |
| 2.8  | Credential capture (keychain read) | `csq` L109-138                                                    | 30    | `src/credentials/keychain.rs` | Complex    | Integration: read from macOS keychain using SHA256 service name. Verify hex-decoded JSON matches.                       |
| 2.9  | Credential capture (file read)     | `csq` L140-148                                                    | 9     | `src/credentials/store.rs`    | Trivial    | Unit: read `.credentials.json`.                                                                                         |
| 2.10 | Canonical save + mirror            | `csq` L155-166                                                    | 12    | `src/credentials/store.rs`    | Moderate   | Unit: verify both `credentials/N.json` and `config-N/.credentials.json` are written.                                    |
| 2.11 | Dashboard atomic write             | `dashboard/refresher.py` L426-441 + `dashboard/oauth.py` L262-283 | 34    | `src/credentials/store.rs`    | Moderate   | Shared with 2.2. Same atomic write function.                                                                            |

---

## 3. Account Identity and Discovery

| #    | v1.x Function                   | Source File                      | Lines | v2.0 Module                 | Complexity | Test Strategy                                                                                                  |
| ---- | ------------------------------- | -------------------------------- | ----- | --------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------- |
| 3.1  | `which_account()`               | `rotation-engine.py` L283-334    | 52    | `src/accounts/identity.rs`  | Moderate   | Unit: mock config dir with `.current-account`, verify fast path. Test fallback chain.                          |
| 3.2  | `csq_account_marker()`          | `rotation-engine.py` L552-579    | 28    | `src/accounts/identity.rs`  | Trivial    | Unit: read `.csq-account`, validate range.                                                                     |
| 3.3  | `write_csq_account_marker()`    | `rotation-engine.py` L582-596    | 15    | `src/accounts/identity.rs`  | Trivial    | Unit: atomic write, read back.                                                                                 |
| 3.4  | `credentials_file_account()`    | `rotation-engine.py` L486-506    | 21    | `src/accounts/identity.rs`  | Moderate   | Unit: match access token across credential files.                                                              |
| 3.5  | `live_credentials_account()`    | `rotation-engine.py` L509-549    | 41    | `src/accounts/identity.rs`  | Moderate   | Unit: match refresh token (race-proof ground truth).                                                           |
| 3.6  | `_match_token_to_account()`     | `rotation-engine.py` L468-483    | 16    | `src/accounts/identity.rs`  | Trivial    | Unit: linear scan of credential files.                                                                         |
| 3.7  | `snapshot_account()`            | `rotation-engine.py` L599-643    | 45    | `src/accounts/snapshot.rs`  | Complex    | Integration: mock process tree + config dir. Verify PID caching (cheap path) and re-snapshot (expensive path). |
| 3.8  | `configured_accounts()`         | `rotation-engine.py` L270-272    | 3     | `src/accounts/discovery.rs` | Trivial    | Unit: scan profiles.json.                                                                                      |
| 3.9  | `get_email()`                   | `rotation-engine.py` L266-267    | 2     | `src/accounts/discovery.rs` | Trivial    | Unit: read from profiles.json.                                                                                 |
| 3.10 | `discover_anthropic_accounts()` | `dashboard/accounts.py` L93-154  | 62    | `src/accounts/discovery.rs` | Moderate   | Unit: mock credentials dir with numbered JSON files. Verify email from profiles.                               |
| 3.11 | `discover_3p_accounts()`        | `dashboard/accounts.py` L157-207 | 51    | `src/accounts/discovery.rs` | Moderate   | Unit: mock settings-zai.json, settings-mm.json.                                                                |
| 3.12 | `load_manual_accounts()`        | `dashboard/accounts.py` L210-246 | 37    | `src/accounts/discovery.rs` | Trivial    | Unit: read dashboard-accounts.json.                                                                            |
| 3.13 | `save_manual_account()`         | `dashboard/accounts.py` L249-308 | 60    | `src/accounts/discovery.rs` | Moderate   | Unit: append to JSON file, verify atomic write.                                                                |
| 3.14 | `discover_all_accounts()`       | `dashboard/accounts.py` L311-345 | 35    | `src/accounts/discovery.rs` | Trivial    | Unit: merge + dedup from all sources.                                                                          |
| 3.15 | `AccountInfo` dataclass         | `dashboard/accounts.py` L23-73   | 51    | `src/accounts/types.rs`     | Trivial    | Unit: `to_dict()` masks token.                                                                                 |
| 3.16 | Profile save (login)            | `csq` L182-192                   | 11    | `src/accounts/profiles.rs`  | Trivial    | Unit: write email + method to profiles.json.                                                                   |

---

## 4. Swap and Rotation

| #   | v1.x Function   | Source File                     | Lines | v2.0 Module              | Complexity | Test Strategy                                                                                                                                                                                                                   |
| --- | --------------- | ------------------------------- | ----- | ------------------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1 | `swap_to()`     | `rotation-engine.py` L840-1014  | 175   | `src/rotation/swap.rs`   | Complex    | Integration: mock config dir, verify `.credentials.json` + `.csq-account` + `.current-account` all updated atomically. Verify delayed verification thread. Parity test: same credential files produce same swap result as v1.x. |
| 4.2 | `pick_best()`   | `rotation-engine.py` L649-678   | 30    | `src/rotation/picker.rs` | Moderate   | Unit: various quota states (all available, some exhausted, all exhausted with different reset times).                                                                                                                           |
| 4.3 | `suggest()`     | `rotation-engine.py` L1020-1045 | 26    | `src/rotation/picker.rs` | Trivial    | Unit: JSON output format.                                                                                                                                                                                                       |
| 4.4 | `auto_rotate()` | `rotation-engine.py` L1051-1093 | 43    | `src/rotation/auto.rs`   | Moderate   | Unit: disabled by default. Callable with `--force`. Verify lock on quota file.                                                                                                                                                  |

---

## 5. Broker / Daemon Refresh

| #   | v1.x Function                                                             | Source File                     | Lines | v2.0 Module            | Complexity | Test Strategy                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------- | ------------------------------- | ----- | ---------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 5.1 | `broker_check()`                                                          | `rotation-engine.py` L1702-1795 | 94    | `src/daemon/broker.rs` | Complex    | Integration: mock HTTP server, verify per-account lock, verify fanout. Concurrent test: 10 tasks, exactly 1 refreshes.                                                         |
| 5.2 | `_broker_recover_from_live()`                                             | `rotation-engine.py` L1634-1699 | 66    | `src/daemon/broker.rs` | Complex    | Integration: set up dead canonical + live sibling with good RT. Verify promotion + successful refresh. Verify rollback on total failure.                                       |
| 5.3 | `_scan_config_dirs_for_account()`                                         | `rotation-engine.py` L1556-1580 | 25    | `src/daemon/fanout.rs` | Trivial    | Unit: mock config dirs with markers.                                                                                                                                           |
| 5.4 | `_fan_out_credentials()`                                                  | `rotation-engine.py` L1583-1606 | 24    | `src/daemon/fanout.rs` | Moderate   | Unit: verify atomic write to each matching config dir. Skip if already in sync.                                                                                                |
| 5.5 | `_broker_failure_flag` / `_broker_mark_failed` / `_broker_mark_recovered` | `rotation-engine.py` L1609-1631 | 23    | `src/daemon/broker.rs` | Trivial    | Unit: touch/remove flag file.                                                                                                                                                  |
| 5.6 | `backsync()`                                                              | `rotation-engine.py` L1366-1503 | 138   | `src/daemon/sync.rs`   | Complex    | Integration: set up live with newer token, verify canonical updated. Monotonicity guard: verify older live does NOT update canonical. Content-match primary + marker fallback. |
| 5.7 | `pullsync()`                                                              | `rotation-engine.py` L1814-1875 | 62    | `src/daemon/sync.rs`   | Moderate   | Integration: set up canonical newer than live, verify live updated. Verify no downgrade.                                                                                       |

---

## 6. Quota and Status

| #   | v1.x Function      | Source File                     | Lines | v2.0 Module               | Complexity | Test Strategy                                                                                                                                         |
| --- | ------------------ | ------------------------------- | ----- | ------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.1 | `update_quota()`   | `rotation-engine.py` L1099-1203 | 105   | `src/quota/update.rs`     | Complex    | Integration: verify payload-hash cursor rejects stale data after swap. Verify `live_credentials_account()` ground truth routing. Verify file locking. |
| 6.2 | `load_state()`     | `rotation-engine.py` L251-263   | 13    | `src/quota/state.rs`      | Trivial    | Unit: load + auto-clear expired windows.                                                                                                              |
| 6.3 | `show_status()`    | `rotation-engine.py` L1218-1245 | 28    | `src/cli/status.rs`       | Trivial    | Unit: format output. Snapshot test against known state.                                                                                               |
| 6.4 | `statusline_str()` | `rotation-engine.py` L1248-1306 | 59    | `src/quota/statusline.rs` | Moderate   | Unit: format string. Test stuck-swap warning. Test broker-failure prefix. Test self-healing stale flag.                                               |
| 6.5 | `fmt_time()`       | `rotation-engine.py` L1208-1215 | 8     | `src/quota/format.rs`     | Trivial    | Unit: edge cases (now, minutes, hours, days).                                                                                                         |

---

## 7. Usage Polling (Dashboard/Daemon)

| #   | v1.x Function                     | Source File                    | Lines | v2.0 Module            | Complexity | Test Strategy                                                                                                                |
| --- | --------------------------------- | ------------------------------ | ----- | ---------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 7.1 | `poll_anthropic_usage()`          | `dashboard/poller.py` L48-101  | 54    | `src/daemon/poller.rs` | Moderate   | Integration: mock HTTP server returning usage JSON. Test 429/401 handling.                                                   |
| 7.2 | `poll_3p_usage()`                 | `dashboard/poller.py` L104-164 | 61    | `src/daemon/poller.rs` | Moderate   | Integration: mock HTTP server returning rate-limit headers.                                                                  |
| 7.3 | `_extract_rate_limit_headers()`   | `dashboard/poller.py` L167-197 | 31    | `src/daemon/poller.rs` | Trivial    | Unit: parse headers from known response.                                                                                     |
| 7.4 | `UsagePoller` (background thread) | `dashboard/poller.py` L200-369 | 170   | `src/daemon/poller.rs` | Complex    | Integration: verify polling intervals, staggered start, exponential backoff on 429, 401 marking. Uses `tokio::time` mocking. |
| 7.5 | `UsageCache`                      | `dashboard/cache.py` L16-93    | 78    | `src/daemon/cache.rs`  | Trivial    | Unit: set/get/TTL expiry/delete. Thread-safety via `RwLock`.                                                                 |

---

## 8. Token Refresh Daemon

| #   | v1.x Function                        | Source File                       | Lines | v2.0 Module                | Complexity | Test Strategy                                                                                                                                              |
| --- | ------------------------------------ | --------------------------------- | ----- | -------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 8.1 | `TokenRefresher` (background thread) | `dashboard/refresher.py` L55-455  | 401   | `src/daemon/refresher.rs`  | Complex    | Integration: mock HTTP server. Verify 5-min check interval. Verify 30-min-ahead refresh trigger. Verify cooldown after failure. Verify monotonicity guard. |
| 8.2 | `_do_refresh()`                      | `dashboard/refresher.py` L251-373 | 123   | `src/daemon/refresher.rs`  | Complex    | Integration: 5-step flow (read, HTTP, re-read, check monotonicity, write). Mock server + temp credential files.                                            |
| 8.3 | `_do_http_refresh()`                 | `dashboard/refresher.py` L375-424 | 50    | `src/credentials/oauth.rs` | Moderate   | Shared with 2.4. Same HTTP refresh function.                                                                                                               |
| 8.4 | `get_token_status()`                 | `dashboard/refresher.py` L144-207 | 64    | `src/daemon/refresher.rs`  | Trivial    | Unit: compute `is_healthy` and `expires_in_seconds` from credential data.                                                                                  |
| 8.5 | `get_all_token_statuses()`           | `dashboard/refresher.py` L209-218 | 10    | `src/daemon/refresher.rs`  | Trivial    | Unit: iterate accounts.                                                                                                                                    |

---

## 9. OAuth Login Flow

| #   | v1.x Function                  | Source File                   | Lines | v2.0 Module             | Complexity | Test Strategy                                                                                                                         |
| --- | ------------------------------ | ----------------------------- | ----- | ----------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 9.1 | `OAuthLogin.start_login()`     | `dashboard/oauth.py` L78-132  | 55    | `src/oauth/pkce.rs`     | Moderate   | Unit: verify PKCE values (verifier length, challenge = SHA256(verifier)). Verify authorize URL contains all required params.          |
| 9.2 | `OAuthLogin.handle_callback()` | `dashboard/oauth.py` L134-207 | 74    | `src/oauth/callback.rs` | Complex    | Integration: mock token exchange endpoint. Verify state consumed (single-use). Verify credential file written.                        |
| 9.3 | `_exchange_code()`             | `dashboard/oauth.py` L209-260 | 52    | `src/oauth/exchange.rs` | Moderate   | Integration: mock HTTP server. Verify request body (grant_type, code, client_id, code_verifier, redirect_uri).                        |
| 9.4 | `_generate_code_verifier()`    | `dashboard/oauth.py` L285-292 | 8     | `src/oauth/pkce.rs`     | Trivial    | Unit: verify length (43 chars), URL-safe base64.                                                                                      |
| 9.5 | `_generate_code_challenge()`   | `dashboard/oauth.py` L294-307 | 14    | `src/oauth/pkce.rs`     | Trivial    | Unit: verify SHA256 + base64url against known test vectors (RFC 7636 Appendix B).                                                     |
| 9.6 | `cmd_login()` (browser flow)   | `csq` L86-195                 | 110   | `src/cli/login.rs`      | Complex    | Integration: verify `claude auth login` invoked with correct `CLAUDE_CONFIG_DIR`. Verify credential capture chain (keychain -> file). |

---

## 10. Provider and Model Management

| #     | v1.x Function                         | Source File                           | Lines | v2.0 Module                 | Complexity | Test Strategy                                                                                                                                                            |
| ----- | ------------------------------------- | ------------------------------------- | ----- | --------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 10.1  | `cmd_setkey()` — provider config      | `csq` L523-752                        | 230   | `src/providers/setkey.rs`   | Complex    | Unit: verify skeleton creation for each provider (claude, mm, zai, ollama). Verify key validation HTTP call. Verify existing profile preserved. Verify primer paths set. |
| 10.2  | Provider skeleton definitions         | `csq` L569-637                        | 69    | `src/providers/catalog.rs`  | Moderate   | Unit: each provider has correct key_fields, auth_type, env vars, model defaults.                                                                                         |
| 10.3  | `cmd_listkeys()`                      | `csq` L802-835                        | 34    | `src/cli/keys.rs`           | Trivial    | Unit: format output with masked keys.                                                                                                                                    |
| 10.4  | `cmd_rmkey()`                         | `csq` L837-846                        | 10    | `src/cli/keys.rs`           | Trivial    | Unit: remove file, error if missing.                                                                                                                                     |
| 10.5  | `cmd_models()` — list all             | `csq` L848-920                        | 73    | `src/cli/models.rs`         | Moderate   | Unit: format with catalog data + active model detection.                                                                                                                 |
| 10.6  | `cmd_models()` — list for provider    | `csq` L922-956                        | 35    | `src/cli/models.rs`         | Moderate   | Unit: catalog models + ollama list integration.                                                                                                                          |
| 10.7  | `cmd_models()` — switch               | `csq` L958-993                        | 36    | `src/cli/models.rs`         | Moderate   | Unit: update all 5 MODEL_KEYS. Verify unknown model rejected. Atomic write.                                                                                              |
| 10.8  | `get_ollama_models()`                 | `csq` L884-892                        | 9     | `src/providers/ollama.rs`   | Trivial    | Integration: verify `ollama list` parsing.                                                                                                                               |
| 10.9  | Model catalog loading                 | `model-catalog.json` + `csq` L869-872 | 4     | `src/providers/catalog.rs`  | Trivial    | Unit: deserialize catalog JSON.                                                                                                                                          |
| 10.10 | Key validation (HTTP probe)           | `csq` L700-732                        | 33    | `src/providers/validate.rs` | Moderate   | Integration: mock server, verify request format. Test 200/401/403/000 responses.                                                                                         |
| 10.11 | JSON auto-repair (truncated profiles) | `csq` L446-472, L647-654              | 20    | `src/providers/repair.rs`   | Moderate   | Unit: truncated JSON with 1-3 missing closing braces. Verify repair + atomic writeback.                                                                                  |

---

## 11. Session Management (Run)

| #    | v1.x Function                          | Source File    | Lines | v2.0 Module                | Complexity | Test Strategy                                                                                |
| ---- | -------------------------------------- | -------------- | ----- | -------------------------- | ---------- | -------------------------------------------------------------------------------------------- |
| 11.1 | `cmd_run()` — account auto-resolution  | `csq` L201-257 | 57    | `src/cli/run.rs`           | Moderate   | Unit: 0 accounts -> vanilla claude. 1 account -> that. 2+ -> error.                          |
| 11.2 | `cmd_run()` — profile auth detection   | `csq` L265-295 | 31    | `src/cli/run.rs`           | Moderate   | Unit: detect ANTHROPIC_AUTH_TOKEN / ANTHROPIC_API_KEY in overlay.                            |
| 11.3 | `cmd_run()` — symlink shared artifacts | `csq` L301-349 | 49    | `src/session/isolation.rs` | Complex    | Integration: verify correct items symlinked vs isolated. Test Windows junctions.             |
| 11.4 | `cmd_run()` — settings merge           | `csq` L428-485 | 58    | `src/session/settings.rs`  | Moderate   | Unit: deep merge of default + overlay. Verify overlay keys override. Test nested dict merge. |
| 11.5 | `cmd_run()` — onboarding flag          | `csq` L415-425 | 11    | `src/session/setup.rs`     | Trivial    | Unit: set `hasCompletedOnboarding` in .claude.json.                                          |
| 11.6 | `cmd_run()` — broker call              | `csq` L383-385 | 3     | `src/cli/run.rs`           | Trivial    | Calls daemon or falls back to synchronous broker.                                            |
| 11.7 | `cmd_run()` — credential copy          | `csq` L392-397 | 6     | `src/session/setup.rs`     | Trivial    | Atomic copy from canonical to config dir.                                                    |
| 11.8 | `cmd_run()` — env stripping            | `csq` L497     | 1     | `src/cli/run.rs`           | Trivial    | Remove ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN before exec.                                  |
| 11.9 | `cmd_run()` — exec claude              | `csq` L499     | 1     | `src/cli/run.rs`           | Trivial    | `std::process::Command::exec()` (Unix) or `spawn + wait` (Windows).                          |

---

## 12. Statusline Hook

| #    | v1.x Function                       | Source File                            | Lines | v2.0 Module             | Complexity | Test Strategy                                                                                                                                                                                                                              |
| ---- | ----------------------------------- | -------------------------------------- | ----- | ----------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 12.1 | `statusline-quota.sh` (full script) | `statusline-quota.sh` L1-148           | 148   | `src/cli/statusline.rs` | Complex    | v2.0 replaces the bash script with a Rust binary invocation: `csq statusline`. Reads CC's JSON from stdin, calls daemon (or does direct computation), outputs formatted string. Parity test: same CC JSON input produces identical output. |
| 12.2 | Snapshot trigger                    | `statusline-quota.sh` L40              | 1     | `src/cli/statusline.rs` | Trivial    | Part of statusline command — calls `snapshot_account()`.                                                                                                                                                                                   |
| 12.3 | Background sync trigger             | `statusline-quota.sh` L52              | 1     | `src/cli/statusline.rs` | Trivial    | Delegates to daemon if running, else spawns background `csq sync`.                                                                                                                                                                         |
| 12.4 | Background quota update             | `statusline-quota.sh` L55              | 1     | `src/cli/statusline.rs` | Trivial    | Delegates to daemon if running.                                                                                                                                                                                                            |
| 12.5 | Context window formatting           | `statusline-quota.sh` L26-31, L121-126 | 12    | `src/cli/statusline.rs` | Trivial    | Unit: parse token counts from CC JSON, format compact.                                                                                                                                                                                     |
| 12.6 | `fmt_tokens()`                      | `statusline-quota.sh` L87-96           | 10    | `src/quota/format.rs`   | Trivial    | Unit: 500 -> "500", 1200 -> "1k", 1500000 -> "1.5M".                                                                                                                                                                                       |
| 12.7 | Git status                          | `statusline-quota.sh` L75-84           | 10    | `src/cli/statusline.rs` | Trivial    | Shell out to `git branch --show-current` + `git diff --quiet`.                                                                                                                                                                             |

---

## 13. HTTP Server (Dashboard API)

| #     | v1.x Function                     | Source File                    | Lines | v2.0 Module         | Complexity | Test Strategy                                                                          |
| ----- | --------------------------------- | ------------------------------ | ----- | ------------------- | ---------- | -------------------------------------------------------------------------------------- |
| 13.1  | `DashboardHandler.do_GET` routing | `dashboard/server.py` L62-91   | 30    | `src/daemon/api.rs` | Moderate   | Use `axum` or `actix-web` router. Unit: verify all routes registered.                  |
| 13.2  | `GET /api/accounts`               | `dashboard/server.py` L107-132 | 26    | `src/daemon/api.rs` | Trivial    | Unit: verify response includes usage + token health.                                   |
| 13.3  | `GET /api/account/{id}/usage`     | `dashboard/server.py` L134-151 | 18    | `src/daemon/api.rs` | Trivial    | Unit: 404 for unknown ID.                                                              |
| 13.4  | `GET /api/refresh`                | `dashboard/server.py` L153-165 | 13    | `src/daemon/api.rs` | Trivial    | Unit: delegates to poller.force_refresh().                                             |
| 13.5  | `GET /api/tokens`                 | `dashboard/server.py` L217-240 | 24    | `src/daemon/api.rs` | Trivial    | Unit: returns all token statuses.                                                      |
| 13.6  | `GET /api/login/{N}`              | `dashboard/server.py` L261-292 | 32    | `src/daemon/api.rs` | Moderate   | Integration: verify PKCE flow initiated, auth_url returned.                            |
| 13.7  | `GET /oauth/callback`             | `dashboard/server.py` L293-330 | 38    | `src/daemon/api.rs` | Complex    | Integration: full callback flow with mock token exchange.                              |
| 13.8  | `POST /api/accounts`              | `dashboard/server.py` L167-215 | 49    | `src/daemon/api.rs` | Moderate   | Unit: validate required fields. Verify account added to live list + poller.            |
| 13.9  | `POST /api/refresh-token/{id}`    | `dashboard/server.py` L242-259 | 18    | `src/daemon/api.rs` | Trivial    | Unit: delegates to refresher.                                                          |
| 13.10 | `create_server()`                 | `dashboard/server.py` L383-466 | 84    | `src/daemon/mod.rs` | Complex    | Integration: verify all subsystems initialized (cache, poller, refresher, oauth).      |
| 13.11 | Static file serving               | `dashboard/server.py` L334-360 | 27    | `src/daemon/api.rs` | Moderate   | Embedded in binary via `include_dir!` or served by Tauri. Path traversal sanitization. |

---

## 14. Installation and Updates

| #    | v1.x Function              | Source File           | Lines | v2.0 Module                          | Complexity | Test Strategy                                                                                                                                                             |
| ---- | -------------------------- | --------------------- | ----- | ------------------------------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 14.1 | `install.sh` (full script) | `install.sh` L1-205   | 205   | `src/cli/install.rs` + shell wrapper | Complex    | v2.0 binary is self-installing: `csq install` creates directories, sets permissions, configures settings.json. Curl-pipe installer downloads binary + runs `csq install`. |
| 14.2 | Settings.json patching     | `install.sh` L151-182 | 32    | `src/cli/install.rs`                 | Moderate   | Unit: verify statusline command set. Verify dead hooks removed.                                                                                                           |
| 14.3 | `_auto_update_bg()`        | `csq` L54-84          | 31    | `src/cli/update.rs`                  | Moderate   | Background version check. Download new binary, verify checksum, atomic replace.                                                                                           |
| 14.4 | `cmd_update()`             | `csq` L755-799        | 45    | `src/cli/update.rs`                  | Moderate   | Foreground update. Download + replace.                                                                                                                                    |
| 14.5 | Migration cleanup          | `install.sh` L125-127 | 3     | `src/cli/install.rs`                 | Trivial    | Remove v1.x artifacts (statusline-command.sh, rotate.md).                                                                                                                 |

---

## 15. CLI Entry Point and Routing

| #    | v1.x Function                       | Source File                     | Lines | v2.0 Module             | Complexity | Test Strategy                                                                            |
| ---- | ----------------------------------- | ------------------------------- | ----- | ----------------------- | ---------- | ---------------------------------------------------------------------------------------- |
| 15.1 | `main()` argument routing           | `csq` L996-1116                 | 121   | `src/main.rs` + `clap`  | Moderate   | Use `clap` derive API. Verify all subcommands registered. Integration: verify help text. |
| 15.2 | No-args default to `run`            | `csq` L999-1001                 | 3     | `src/main.rs`           | Trivial    | `clap` default subcommand.                                                               |
| 15.3 | Numeric first arg = `run N`         | `csq` L1003-1005                | 3     | `src/main.rs`           | Moderate   | Custom `clap` parsing to detect leading numeric argument.                                |
| 15.4 | `rotation-engine.py main()` routing | `rotation-engine.py` L1881-1960 | 80    | N/A (absorbed into CLI) | N/A        | All engine subcommands become CLI subcommands or daemon internal methods.                |

---

## Summary Statistics

| Category             | Functions | Trivial | Moderate | Complex | Total v1.x Lines |
| -------------------- | --------- | ------- | -------- | ------- | ---------------- |
| 1. Platform          | 17        | 5       | 4        | 8       | ~230             |
| 2. Credentials       | 11        | 4       | 5        | 2       | ~295             |
| 3. Account Identity  | 16        | 7       | 5        | 4       | ~460             |
| 4. Swap/Rotation     | 4         | 1       | 2        | 1       | ~274             |
| 5. Broker/Sync       | 7         | 2       | 2        | 3       | ~432             |
| 6. Quota/Status      | 5         | 3       | 1        | 1       | ~213             |
| 7. Usage Polling     | 5         | 2       | 2        | 1       | ~394             |
| 8. Token Refresh     | 5         | 2       | 1        | 2       | ~648             |
| 9. OAuth Login       | 6         | 2       | 2        | 2       | ~313             |
| 10. Providers/Models | 11        | 3       | 6        | 2       | ~562             |
| 11. Session (Run)    | 9         | 5       | 3        | 1       | ~216             |
| 12. Statusline       | 7         | 5       | 0        | 2       | ~183             |
| 13. HTTP Server      | 11        | 5       | 3        | 3       | ~359             |
| 14. Install/Update   | 5         | 1       | 3        | 1       | ~316             |
| 15. CLI Routing      | 4         | 1       | 2        | 0       | ~207             |
| **TOTAL**            | **123**   | **48**  | **41**   | **33**  | **~5,102**       |

### Estimated v2.0 Rust Lines

Applying typical Python-to-Rust expansion factors:

- Trivial: 1.2x (type annotations, match statements)
- Moderate: 1.5x (error handling, Result types, trait implementations)
- Complex: 1.8x (async, generics, platform conditional compilation)

Estimated total: **~8,500-10,000 lines of Rust** (excluding tests and frontend).

### Removed Functions (No v2.0 Equivalent)

| v1.x Function                      | Reason                                      |
| ---------------------------------- | ------------------------------------------- |
| `_find_python()` / `_python_cmd()` | No Python dependency in v2.0                |
| `auto-rotate-hook.sh`              | Dead code in v1.x (exit 0 no-op)            |
| All inline Python in `csq` bash    | Logic absorbed into Rust modules            |
| `suggest_install()` (install.sh)   | v2.0 binary has no prerequisites to suggest |

### New Functions (No v1.x Equivalent)

| v2.0 Function                          | Module                    | Rationale                   |
| -------------------------------------- | ------------------------- | --------------------------- |
| Daemon lifecycle (start/stop/health)   | `src/daemon/lifecycle.rs` | v1.x has no daemon          |
| Unix socket / named pipe server        | `src/daemon/ipc.rs`       | CLI-to-daemon communication |
| System tray integration                | `src/desktop/tray.rs`     | New desktop feature         |
| Tauri window management                | `src/desktop/window.rs`   | New desktop feature         |
| Auto-update with checksum verification | `src/cli/update.rs`       | v1.x uses raw curl          |
| `csq doctor` diagnostic command        | `src/cli/doctor.rs`       | New CLI feature             |
| Shell completions generator            | `src/cli/completions.rs`  | New CLI feature             |
| `--json` output for all commands       | Throughout CLI            | New CLI feature             |

---

## Priority Matrix

Every feature grouped by functional area with priority assignment, effort estimate, and dependency chain. Priority definitions:

- **P0 (Launch Blocker)**: Without this, v2.0 cannot ship. Includes all v1.x parity features that users depend on daily.
- **P1 (Fast-Follow)**: Ship within 1-2 sessions after P0. New features that define the v2.0 value proposition (daemon, tray, dashboard).
- **P2 (Future)**: Nice-to-have. Can ship in a point release. Not required for the initial v2.0 announcement.

Effort is measured in **autonomous sessions**. One session = one focused execution cycle with full agent team deployment, typically 3-6 hours wall clock. The 10x multiplier applies (mature COC institutional knowledge for this project).

---

### P0 — Launch Blockers

These features constitute functional parity with v1.x CLI. Without them, existing csq users cannot migrate.

| Area                  | Feature Group                                                    | Scope Ref    | Effort (sessions) | Depends On      |
| --------------------- | ---------------------------------------------------------------- | ------------ | :---------------: | --------------- |
| Platform Abstraction  | Platform detection, file permissions, atomic writes              | 1.1-1.6      |        0.5        | --              |
| Platform Abstraction  | File locking (POSIX + Windows)                                   | 1.7-1.11     |        0.5        | --              |
| Platform Abstraction  | Process detection (PID alive, CC PID walk)                       | 1.12-1.17    |         1         | --              |
| Credential Management | Load/save/validate credentials, AccountNum newtype               | 2.1-2.3      |        0.5        | Platform        |
| Credential Management | OAuth token refresh (HTTP POST, atomic write, monotonicity)      | 2.4          |         1         | Platform, Creds |
| Credential Management | Keychain integration (macOS, Linux, Windows via `keyring` crate) | 2.5-2.8      |         1         | Platform, Creds |
| Credential Management | Credential file write + mirror                                   | 2.9-2.11     |        0.5        | Platform, Creds |
| Account Identity      | which_account(), markers, token matching                         | 3.1-3.6      |         1         | Creds           |
| Account Identity      | snapshot_account() with process tree walk                        | 3.7          |        0.5        | Platform, Acct  |
| Account Identity      | Account discovery (Anthropic, 3P, manual, combined)              | 3.8-3.16     |         1         | Creds           |
| Swap and Rotation     | swap_to() with verification + delayed check                      | 4.1          |         1         | Creds, Acct     |
| Swap and Rotation     | pick_best(), suggest()                                           | 4.2-4.3      |        0.5        | Quota           |
| Broker / Sync         | broker_check() with per-account lock + fanout                    | 5.1, 5.3-5.5 |        1.5        | Creds, Acct     |
| Broker / Sync         | Recovery from dead refresh token (live sibling promotion)        | 5.2          |         1         | Broker          |
| Broker / Sync         | backsync() + pullsync() with monotonicity                        | 5.6-5.7      |         1         | Creds, Acct     |
| Quota and Status      | update_quota() with payload-hash cursor                          | 6.1          |         1         | Acct            |
| Quota and Status      | load_state(), show_status(), statusline_str()                    | 6.2-6.5      |        0.5        | Quota           |
| Statusline Hook       | csq statusline (replaces statusline-quota.sh)                    | 12.1-12.7    |         1         | Quota, Acct     |
| Session Management    | csq run (auto-resolve, symlinks, settings merge, broker, exec)   | 11.1-11.9    |        1.5        | All above       |
| Provider Management   | setkey, listkeys, rmkey, model switch                            | 10.1-10.11   |         1         | Platform        |
| CLI Entry Point       | clap routing, subcommands, numeric-first-arg                     | 15.1-15.4    |         1         | --              |
| Install / Update      | csq install (self-installing binary), settings patching          | 14.1-14.2    |         1         | Platform        |
| Install / Update      | Auto-update + manual update with checksum                        | 14.3-14.5    |        0.5        | Platform        |
| **P0 Total**          |                                                                  |              |      **19**       |                 |

### P1 — Fast-Follow (v2.0 Value Proposition)

These features define what makes v2.0 worth the rewrite. They ship within 1-2 weeks of P0.

| Area              | Feature Group                                           | Scope Ref | Effort (sessions) | Depends On          |
| ----------------- | ------------------------------------------------------- | --------- | :---------------: | ------------------- |
| Daemon            | Daemon lifecycle (start, stop, health check, PID file)  | New       |         1         | Platform            |
| Daemon            | Unix socket / named pipe IPC server (HTTP over socket)  | New       |        1.5        | Daemon lifecycle    |
| Daemon            | CLI-to-daemon delegation (status, swap, statusline)     | New       |         1         | IPC                 |
| Daemon            | Background token refresh (replaces broker subprocess)   | 8.1-8.5   |        1.5        | Daemon, Creds       |
| Daemon            | Background usage polling (Anthropic + 3P)               | 7.1-7.5   |         1         | Daemon, Creds       |
| Daemon            | In-memory cache with TTL                                | 7.5       |        0.5        | Daemon              |
| HTTP API          | Dashboard API (accounts, usage, tokens, refresh, login) | 13.1-13.9 |        1.5        | Daemon, Poller      |
| HTTP API          | Server lifecycle + subsystem initialization             | 13.10     |        0.5        | HTTP API            |
| OAuth (Dashboard) | PKCE flow (start, callback, exchange) via browser       | 9.1-9.5   |         1         | HTTP API, Creds     |
| Desktop (Tauri)   | Tauri project scaffolding + Svelte frontend skeleton    | New       |         1         | --                  |
| Desktop (Tauri)   | System tray with account status + quick-swap menu       | New       |         1         | Daemon, Tauri       |
| Desktop (Tauri)   | Dashboard UI: account list, usage bars, token health    | New       |        1.5        | HTTP API, Tauri     |
| Desktop (Tauri)   | Dashboard UI: OAuth login from browser                  | New       |         1         | OAuth, Dashboard UI |
| Desktop (Tauri)   | Tauri IPC allowlist + CSP + isolation                   | New       |        0.5        | Tauri               |
| Desktop (Tauri)   | Tauri auto-update (Ed25519 signed)                      | New       |        0.5        | Tauri               |
| **P1 Total**      |                                                         |           |      **14**       |                     |

### P2 — Future (Point Releases)

Nice-to-have features that improve the experience but are not required for v2.0 launch.

| Area          | Feature Group                                           | Scope Ref      | Effort (sessions) | Depends On    |
| ------------- | ------------------------------------------------------- | -------------- | :---------------: | ------------- |
| CLI Polish    | `csq doctor` diagnostic command                         | New            |        0.5        | Daemon        |
| CLI Polish    | Shell completions (bash, zsh, fish, PowerShell)         | New            |        0.5        | CLI           |
| CLI Polish    | `--json` output for all commands                        | New            |         1         | CLI           |
| Rotation      | Pre-emptive rotation (swap before hitting limit)        | 4.4 (enhanced) |         1         | Daemon, Quota |
| Distribution  | Homebrew tap formula                                    | New            |        0.5        | CI/CD         |
| Distribution  | Scoop manifest (Windows)                                | New            |        0.5        | CI/CD         |
| Distribution  | .deb and .rpm packages                                  | New            |        0.5        | CI/CD         |
| Distribution  | Code signing (macOS notarization, Windows Authenticode) | New            |         1         | CI/CD         |
| Security      | Certificate pinning for platform.claude.com             | S22            |        0.5        | Creds         |
| Security      | Token encryption at rest (beyond file permissions)      | S23            |         1         | Creds         |
| Security      | mlock for token pages in memory                         | S24            |        0.5        | Creds         |
| Observability | Structured logging with tracing crate                   | OBS-001        |        0.5        | --            |
| Observability | Daemon health endpoint with metrics                     | OBS-002/003    |        0.5        | Daemon        |
| Dashboard     | Settings/profile editor in dashboard                    | New            |         1         | Dashboard UI  |
| Dashboard     | Dark/light theme + responsive design                    | New            |        0.5        | Dashboard UI  |
| **P2 Total**  |                                                         |                |       **9**       |               |

---

## Dependency Graph

```
                    Platform Abstraction (P0)
                    /          |          \
                   v           v           v
            File Locking    Atomic I/O    Process Detection
                   \          |          /
                    v         v         v
                  Credential Management (P0)
                  /        |         \
                 v         v          v
          Account      Keychain     OAuth Token
          Identity     Integration   Refresh
            |              |            |
            v              v            v
         Swap/Rotation   Broker/Sync  Quota/Status
              |              |            |
              v              v            v
           Session Mgmt  Statusline   Provider Mgmt
              |              |            |
              v              v            v
           CLI Entry Point + Install/Update
              |
              | ---- P0 Complete ----
              v
           Daemon Lifecycle (P1)
           /         |         \
          v          v          v
       IPC Server  Token     Usage
                   Refresher  Poller
          |          |          |
          v          v          v
       HTTP API    In-Memory Cache
          |
          v
       Tauri Desktop (P1)
       /       |        \
      v        v         v
   Tray    Dashboard  Auto-Update
           UI
              |
              v
       OAuth Login via Dashboard
              |
              | ---- P1 Complete ----
              v
       P2 Features (parallel, independent)
```

---

## Cross-Cutting Concerns

These apply to every module and are not standalone features:

| Concern               | Implementation                                                                  | Applied To         |
| --------------------- | ------------------------------------------------------------------------------- | ------------------ |
| Secret types          | `AccessToken`, `RefreshToken` newtypes with `Display` masking + `secrecy` crate | All credential ops |
| AccountNum validation | Newtype with `TryFrom<u16>`, range 1..MAX_ACCOUNTS                              | All account ops    |
| Error handling        | `thiserror` crate for typed errors, `anyhow` for CLI top-level                  | All modules        |
| Atomic file writes    | `tempfile::NamedTempFile::persist()` everywhere                                 | All file mutations |
| File permissions      | `_secure_file()` equivalent after every credential write                        | All credential ops |
| Structured logging    | `tracing` crate with `CSQ_LOG` env control                                      | All modules        |
| Platform conditionals | `cfg(target_os)` for OS-specific code paths                                     | Platform layer     |
| Test parity           | v1.x-compatible test fixtures: same input produces same output                  | Credential, Quota  |

---

## Effort Summary

| Priority  | Sessions | Wall Clock (estimated) | Notes                                            |
| --------- | :------: | :--------------------: | ------------------------------------------------ |
| P0        |    19    |        5-7 days        | Parallelizable into 3 streams (see impl plan)    |
| P1        |    14    |        4-5 days        | Sequential dependency on P0 core; Tauri parallel |
| P2        |    9     |        2-3 days        | Fully parallel, independent features             |
| **Total** |  **42**  |     **11-15 days**     | Autonomous execution, not human-days             |

Wall clock assumes 3-4 parallel sessions per day with mature COC institutional knowledge. The critical path runs through Platform -> Credentials -> Broker -> Session -> CLI (P0), then Daemon -> HTTP API -> Dashboard (P1). Tauri scaffolding and frontend can proceed in parallel with daemon work.
