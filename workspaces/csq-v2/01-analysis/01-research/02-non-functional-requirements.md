# csq v2.0 — Non-Functional Requirements

Performance, security, reliability, cross-platform, and distribution requirements derived from v1.x operational characteristics and the v2.0 vision.

---

## 1. Performance

### 1.1 Startup Time

| Metric                   | v1.x Baseline                                                       | v2.0 Target | Rationale                                                             |
| ------------------------ | ------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------- |
| CLI cold start           | ~300ms (bash + Python interpreter load)                             | <50ms       | Rust binary, no interpreter. Ollama achieves ~20ms for `ollama list`. |
| `csq run N` to CC launch | ~1.5s (Python broker + credential copy + settings merge + symlinks) | <200ms      | All operations in-process, no subprocess spawning                     |
| `csq swap N`             | ~100ms (Python engine invocation)                                   | <20ms       | Direct function call within running daemon or CLI                     |
| `csq status`             | ~200ms (Python load + file reads)                                   | <30ms       | In-memory state if daemon running, file reads if CLI-only             |
| Statusline render        | ~400ms (3 Python subprocesses: snapshot, sync, update)              | <50ms       | Single IPC call to daemon, or fast binary invocation                  |
| Dashboard page load      | ~500ms (Python HTTP + JS)                                           | <100ms      | Compiled Rust backend + bundled Svelte frontend                       |

### 1.2 Memory

| Component            | v1.x Baseline                             | v2.0 Target | Rationale                                                    |
| -------------------- | ----------------------------------------- | ----------- | ------------------------------------------------------------ |
| CLI invocation       | ~30MB (Python interpreter per invocation) | <5MB        | Rust binary, no runtime                                      |
| Dashboard daemon     | ~50MB (Python HTTP server + threads)      | <20MB       | Rust async runtime (tokio) + Tauri webview                   |
| Per-account overhead | ~2MB (cached JSON in Python dicts)        | <200KB      | Typed structs, no dynamic allocation overhead                |
| System tray idle     | N/A (no tray in v1.x)                     | <30MB       | Tauri + webview base cost, measured against Tauri benchmarks |

### 1.3 Binary Size

| Target              | Size                                       | Rationale                                                        |
| ------------------- | ------------------------------------------ | ---------------------------------------------------------------- |
| CLI-only binary     | <10MB                                      | Comparable to ripgrep (~6MB). No webview overhead.               |
| Desktop app (Tauri) | <25MB (macOS .app), <15MB (Linux AppImage) | Tauri apps typically 5-15MB. System webview adds no binary size. |
| Installer           | <5MB download                              | Compressed binary + assets                                       |

### 1.4 Concurrency

| Metric                         | v1.x Baseline                                | v2.0 Target                                    |
| ------------------------------ | -------------------------------------------- | ---------------------------------------------- |
| Concurrent terminals supported | 15+ (tested, with file locking)              | 50+ (per-account async locks)                  |
| Concurrent statusline renders  | 15 Python processes spawned per render cycle | 1 IPC call per terminal to daemon              |
| Token refresh concurrency      | Per-account try-lock (fcntl/mutex)           | Per-account async Mutex, zero process spawning |

---

## 2. Security

### 2.1 Credential Storage

| Requirement                         | v1.x Implementation                                            | v2.0 Requirement                                                                                    |
| ----------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| SEC-001: Primary credential store   | `credentials/N.json` files with `0o600`                        | Same file layout (backwards compatible). Consider `keyring` crate for OS keychain on all platforms. |
| SEC-002: macOS keychain integration | SHA256-based service name, hex-encoded JSON via `security` CLI | Use `keyring` crate or Security.framework FFI directly. Eliminate 3-second subprocess timeout.      |
| SEC-003: File permissions           | `chmod 0o600` on credential files, no-op on Windows            | Same. Use `std::fs::set_permissions` on Unix. On Windows, rely on user-profile ACL.                 |
| SEC-004: Atomic writes              | temp file + `os.replace()` with Windows retry                  | `tempfile` crate + `std::fs::rename()`. Windows: retry with backoff.                                |
| SEC-005: No secrets in logs         | Token prefixes only (`token[:8]...`)                           | Same. Use a `SecretString` wrapper type that implements `Display` with masking.                     |
| SEC-006: No secrets in process args | Keys read from stdin, not CLI args                             | Same. Additionally, the daemon API must not accept tokens as URL parameters.                        |
| SEC-007: Input validation           | `_validate_account()` — range 1..999, digits only              | Same validation. Apply to all path-constructing inputs. Prevent path traversal.                     |
| SEC-008: No shell=True              | v1.x uses subprocess arrays only                               | Rust `Command` API is array-based by default. No shell interpretation.                              |

### 2.2 Network Security

| Requirement                             | Details                                                                                                       |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| SEC-009: HTTPS only                     | All Anthropic API calls over HTTPS. Third-party providers must also use HTTPS except localhost (Ollama).      |
| SEC-010: PKCE for OAuth                 | Continue using S256 code challenge. Generate verifier with `rand::OsRng`.                                     |
| SEC-011: CSRF state parameter           | Single-use state tokens for OAuth callbacks. Must be consumed on use.                                         |
| SEC-012: Local-only binding             | Dashboard/daemon HTTP server binds to `127.0.0.1` only. No authentication required for local access.          |
| SEC-013: No credential exposure via API | API responses mask tokens (prefix only). Full tokens never leave the daemon except in credential file writes. |

### 2.3 Cryptographic Requirements

| Requirement                    | Details                                                                                             |
| ------------------------------ | --------------------------------------------------------------------------------------------------- |
| SEC-014: PKCE code verifier    | 32 bytes from CSPRNG, base64url-encoded (43 chars per RFC 7636)                                     |
| SEC-015: PKCE code challenge   | SHA-256 of verifier, base64url-encoded without padding                                              |
| SEC-016: State parameter       | 32 bytes from CSPRNG, base64url-encoded                                                             |
| SEC-017: Keychain service name | SHA-256 of NFC-normalized config dir path, first 8 hex chars. Must be identical to CC's derivation. |

---

## 3. Reliability

### 3.1 Daemon Lifecycle

| Requirement                          | Details                                                                                                                                                 |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| REL-001: Daemon auto-start           | Daemon starts on first `csq run` or when tray app launches. CLI commands start daemon if not running.                                                   |
| REL-002: Daemon auto-restart         | If daemon crashes, next CLI invocation or tray app restart recovers. PID file with liveness check.                                                      |
| REL-003: Graceful shutdown           | SIGTERM/SIGINT handler flushes state, completes in-flight refreshes, releases locks.                                                                    |
| REL-004: CLI fallback without daemon | All CLI commands must work without the daemon running (direct file operations, like v1.x). Daemon provides performance, not correctness.                |
| REL-005: Lock recovery               | Stale lock files (from crashed processes) must be detectable and recoverable. Use PID-in-lockfile or advisory locks that auto-release on process death. |

### 3.2 Token Refresh Reliability

| Requirement                                | Details                                                                                                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| REL-006: Broker equivalence                | The daemon replaces the broker pattern. Single refresher per account, with per-account locks. Must prevent N terminals from racing on the same refresh. |
| REL-007: Recovery from dead refresh tokens | When canonical RT is dead (CC won a race), try each live sibling's RT. Identical to `_broker_recover_from_live()`.                                      |
| REL-008: Failure signaling                 | Visible indicator when token refresh fails permanently. Equivalent to `LOGIN-NEEDED` in statusline.                                                     |
| REL-009: Monotonicity guard                | Never downgrade credentials. Only write if new `expiresAt` is strictly newer than current.                                                              |
| REL-010: Fanout after refresh              | After successful refresh, push new credentials to all config dirs running that account.                                                                 |

### 3.3 Data Integrity

| Requirement                       | Details                                                                                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| REL-011: Atomic credential writes | All credential file writes must be atomic (temp + rename). No partial writes visible to readers.                                                |
| REL-012: Quota file locking       | Concurrent quota updates from multiple terminals must not corrupt `quota.json`.                                                                 |
| REL-013: Stale data rejection     | Payload-hash cursor mechanism from v1.x must be preserved. Stale rate-limits from pre-swap account must not be attributed to post-swap account. |
| REL-014: Backwards compatibility  | v2.0 must read v1.x credential files, profiles, and quota state without migration. v1.x and v2.0 installations must coexist during transition.  |

---

## 4. Cross-Platform

### 4.1 Platform Requirements

| Platform             | CLI                 | Desktop App                         | Keychain                        | File Locking                   | Notes                                                    |
| -------------------- | ------------------- | ----------------------------------- | ------------------------------- | ------------------------------ | -------------------------------------------------------- |
| macOS (arm64)        | Required            | Required (Tauri + system WebKit)    | Security.framework              | `flock()`                      | Primary platform. App signing required for distribution. |
| macOS (x86_64)       | Required            | Required                            | Security.framework              | `flock()`                      | Universal binary or separate build.                      |
| Linux (x86_64)       | Required            | Required (Tauri + system WebKitGTK) | `libsecret` via `keyring` crate | `flock()`                      | AppImage for desktop. WebKitGTK dependency.              |
| Linux (arm64)        | Required            | Best-effort                         | `libsecret`                     | `flock()`                      | Raspberry Pi / ARM servers. CLI priority.                |
| Windows 10+ (x86_64) | Required            | Required (Tauri + WebView2)         | Windows Credential Manager      | Named mutex via `CreateMutexW` | WebView2 ships with Windows 10+.                         |
| WSL                  | Required (CLI only) | Not required                        | File-based fallback             | `flock()`                      | Desktop app not meaningful in WSL.                       |
| Git Bash on Windows  | Required (CLI only) | Not required                        | File-based fallback             | Named mutex                    | Directory junctions instead of symlinks.                 |

### 4.2 Platform-Specific Behaviors

| Behavior            | macOS                           | Linux                        | Windows                                          |
| ------------------- | ------------------------------- | ---------------------------- | ------------------------------------------------ |
| Config dir symlinks | `ln -s`                         | `ln -s`                      | Directory junctions (`mklink /J`) or file copies |
| File permissions    | `chmod 0o600`                   | `chmod 0o600`                | No-op (NTFS ACL defaults)                        |
| Process tree walk   | `ps -p PID -o ppid=,command=`   | Same                         | `CreateToolhelp32Snapshot`                       |
| PID alive check     | `kill(pid, 0)`                  | Same                         | `OpenProcess + GetExitCodeProcess`               |
| Python command      | `python3`                       | `python3`                    | `python3`, `python`, or `py`                     |
| Keychain service    | `security add-generic-password` | `secret-tool` or `libsecret` | `CredWrite/CredRead`                             |

### 4.3 File Layout Compatibility

v2.0 must use the same file layout as v1.x for coexistence:

```
~/.claude/
  accounts/
    credentials/
      1.json, 2.json, ...     (canonical OAuth credentials)
      N.broker-failed          (broker failure flag)
      N.lock                   (per-canonical lock)
      N.refresh-lock           (per-account refresh lock)
    config-1/, config-2/, ...  (per-terminal config dirs)
      .credentials.json        (live credentials)
      .csq-account             (account marker)
      .current-account         (statusline display)
      .live-pid                (snapshot PID cache)
      .quota-cursor            (stale-data guard)
      .claude.json             (CC config)
      settings.json            (merged settings)
    profiles.json              (email-account mapping)
    quota.json                 (quota state)
    rotation-engine.py         (v1.x engine — coexist)
    statusline-quota.sh        (v1.x statusline — coexist)
    model-catalog.json         (model catalog)
    3p-model-primer.md         (system prompt primer)
    3p-model-primer-prepend.md (prepend primer)
  settings.json                (canonical default settings)
  settings-mm.json             (provider profile overlays)
  settings-zai.json
  settings-claude.json
  settings-ollama.json
```

---

## 5. Distribution

### 5.1 Package Formats

| Platform | Format           | Contents                                         | Install Method                                   |
| -------- | ---------------- | ------------------------------------------------ | ------------------------------------------------ |
| macOS    | `.dmg`           | App bundle + CLI symlink to `/usr/local/bin/csq` | Drag to Applications. CLI available immediately. |
| macOS    | Homebrew tap     | Formula with binary download                     | `brew install terrene-foundation/tap/csq`        |
| Linux    | AppImage         | Self-contained desktop app                       | Download, `chmod +x`, run                        |
| Linux    | `.deb`           | CLI + desktop app                                | `dpkg -i` / `apt install`                        |
| Linux    | `.rpm`           | CLI + desktop app                                | `rpm -i` / `dnf install`                         |
| Windows  | `.msi` installer | Desktop app + CLI in PATH                        | Standard Windows installer                       |
| Windows  | Scoop manifest   | CLI-only                                         | `scoop install csq`                              |
| All      | `curl \| sh`     | CLI binary only (no desktop app)                 | `curl -sSL .../install.sh \| sh`                 |

### 5.2 Auto-Update

| Requirement                       | Details                                                                                  |
| --------------------------------- | ---------------------------------------------------------------------------------------- |
| DIST-001: Background update check | Check for new version on startup (like v1.x `_auto_update_bg()`). 3-second timeout.      |
| DIST-002: In-place binary update  | Download new binary, verify checksum, atomic replace.                                    |
| DIST-003: Desktop app update      | Tauri updater plugin (built-in). JSON manifest at `https://terrene.dev/csq/update.json`. |
| DIST-004: Rollback                | Keep previous binary version for manual rollback.                                        |
| DIST-005: Version reporting       | `csq --version` shows version, build date, commit hash.                                  |

### 5.3 Code Signing

| Platform | Requirement                 | Details                                                               |
| -------- | --------------------------- | --------------------------------------------------------------------- |
| macOS    | Required for Gatekeeper     | Apple Developer ID certificate. Notarization required.                |
| Windows  | Recommended for SmartScreen | Authenticode signature. EV code signing reduces SmartScreen warnings. |
| Linux    | Not required                | GPG signature for package repos.                                      |

### 5.4 CI/CD Pipeline

| Stage   | Details                                                                                          |
| ------- | ------------------------------------------------------------------------------------------------ |
| Build   | GitHub Actions: cross-compile for macOS (arm64, x86_64), Linux (x86_64, arm64), Windows (x86_64) |
| Test    | Unit tests + integration tests on all platforms. Credential parity tests against v1.x.           |
| Sign    | macOS notarization. Windows Authenticode.                                                        |
| Release | GitHub Releases with checksums. Update manifest for Tauri updater.                               |
| Publish | Homebrew tap update. Scoop manifest update.                                                      |

---

## 6. Observability

| Requirement                     | Details                                                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| OBS-001: Structured logging     | `tracing` crate with configurable levels. Default: WARN to stderr. `CSQ_LOG=debug` for verbose.                           |
| OBS-002: Daemon health endpoint | GET `/api/health` returns uptime, accounts monitored, last refresh times.                                                 |
| OBS-003: Metrics                | Account count, refresh success/failure counts, polling intervals, lock contention events. Exposed via health endpoint.    |
| OBS-004: Error diagnostics      | `csq doctor` command: checks file permissions, credential validity, daemon status, network connectivity, keychain access. |

---

## 7. Accessibility

| Requirement                      | Details                                                                                   |
| -------------------------------- | ----------------------------------------------------------------------------------------- |
| ACC-001: CLI-first design        | Every feature accessible via CLI. Desktop app is a convenience layer, not a requirement.  |
| ACC-002: Machine-readable output | `--json` flag for all status/query commands. Enables scripting and integration.           |
| ACC-003: Shell completions       | Generate completions for bash, zsh, fish, PowerShell via `clap_complete`.                 |
| ACC-004: Offline operation       | CLI works fully offline (no update checks fail, no polling errors). Graceful degradation. |
