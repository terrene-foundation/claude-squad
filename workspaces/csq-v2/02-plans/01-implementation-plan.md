# csq v2.0 — Implementation Plan

Phased delivery plan for the Rust + Tauri rewrite. Each phase has defined scope, dependencies, critical path, test strategy, exit criteria, and effort in autonomous sessions.

**Primary inputs**: Scope Matrix (04-scope-matrix.md), ADRs (03-architecture-decision-records.md), Security Analysis (05-security-analysis.md).

---

## Guiding Principles

1. **v1.x parity first, then new features.** No user should be worse off after upgrading. Every `csq` command that works today works identically in v2.0 before any new capability ships.

2. **Bottom-up build order.** The scope matrix dependency graph dictates the sequence: Platform Abstraction -> Credential Management -> Account Identity -> Swap/Broker/Quota -> Session/CLI -> Daemon -> Desktop. Each layer is testable in isolation before the next layer begins.

3. **Single binary from day one.** ADR-004 mandates that CLI, daemon, and desktop modes are the same binary. The Cargo workspace is structured so that `cargo build` produces one binary with feature flags controlling what is included.

4. **Daemon-optional operation.** Every CLI command works without a running daemon (direct file access, like v1.x). When a daemon is running, commands delegate to it for speed and coordination. This means P0 ships a fully functional CLI that does not require the daemon at all.

5. **Test parity.** For every function that has a v1.x equivalent, there is a test that feeds the same input to both implementations and verifies identical output. This is the primary correctness guarantee for the migration.

---

## Phase 0: Project Scaffolding

### Scope

Set up the Cargo workspace, Tauri project skeleton, CI pipeline, and development tooling. No business logic. The output is a repository where `cargo build` succeeds, `cargo test` runs (with zero tests), and CI produces binaries for macOS, Linux, and Windows.

### Deliverables

| Deliverable | Details |
| --- | --- |
| Cargo workspace | Root `Cargo.toml` with `csq-core` (library), `csq-cli` (binary), `csq-desktop` (Tauri binary) members |
| Feature flags | `default = ["cli"]`, `desktop = ["tauri", "tauri-plugin-*"]`. CLI-only builds exclude all Tauri code. |
| Tauri skeleton | `src-tauri/` with `tauri.conf.json`, empty Svelte frontend in `src/`, `cargo tauri dev` launches a blank window |
| CI pipeline | GitHub Actions: `cargo check` + `cargo test` + `cargo clippy` on push. `cargo tauri build` for release tags. Matrix: macOS-arm64, macOS-x86_64, Linux-x86_64, Windows-x86_64. |
| Linting | `clippy::pedantic` enabled. `rustfmt` with project config. `cargo deny` for license and vulnerability audit. |
| Dev tooling | `just` (or `cargo-make`) recipes: `build`, `test`, `lint`, `dev` (Tauri dev server), `release` |
| Error handling setup | `thiserror` for typed errors, `anyhow` for CLI top-level. Error types defined in `csq-core/src/error.rs`. |
| Logging setup | `tracing` crate with `CSQ_LOG` environment variable. `tracing-subscriber` with `EnvFilter`. |
| Cross-cutting types | `AccountNum` newtype (validated 1..MAX_ACCOUNTS), `AccessToken` / `RefreshToken` secret newtypes with masked `Display` |

### Dependencies

None. This is the starting point.

### Critical Path

Tauri project initialization and CI configuration. The CI matrix for Windows requires a Windows runner with WebView2 SDK — verify this works before proceeding.

### Test Strategy

- `cargo build` succeeds on all three platforms
- `cargo test` runs (zero tests, zero failures)
- `cargo clippy` passes with zero warnings
- CI pipeline completes end-to-end on all matrix targets
- `cargo tauri dev` launches a window on macOS (manual verification)

### Exit Criteria

- [ ] `cargo build --features cli` produces a `csq` binary under 5MB
- [ ] `cargo tauri build` produces platform installers
- [ ] CI green on all four matrix targets (macOS-arm64, macOS-x86_64, Linux-x86_64, Windows-x86_64)
- [ ] `AccountNum::try_from(0)` returns `Err`, `AccountNum::try_from(1)` returns `Ok`
- [ ] `AccessToken` displays as `sk-ant-...xxxx` (masked)

### Effort

**1 autonomous session.** Parallelizable: CI setup can proceed while Cargo workspace is configured.

---

## Phase 1: Core Credential Management

### Scope

The hardest phase. Implements the Platform Abstraction Layer (scope 1.1-1.17) and Credential Management (scope 2.1-2.11). These 28 functions contain 10 Complex-rated items and are the foundation for everything else.

This phase also includes Account Identity (scope 3.1-3.16) and the Broker/Sync layer (scope 5.1-5.7), because credential management without account detection and token refresh coordination is untestable in any meaningful way.

### Deliverables

#### Stream A: Platform Abstraction (scope 1.1-1.17)

| Module | Functions | Complexity |
| --- | --- | --- |
| `platform/mod.rs` | Platform detection (compile-time `cfg`) | Trivial |
| `platform/fs.rs` | `secure_file()`, `atomic_replace()` | Trivial + Moderate |
| `platform/lock.rs` | `lock_file()`, `try_lock_file()`, `unlock_file()` — POSIX via `flock`, Windows via named mutex | 2x Complex, 2x Moderate, 1x Trivial |
| `platform/process.rs` | `is_pid_alive()`, `find_cc_pid()`, `is_cc_command()` — POSIX via `/proc` or `kill(0)`, Windows via `CreateToolhelp32Snapshot` | 3x Complex, 1x Moderate, 1x Trivial |
| `platform/win32.rs` | FFI declarations for Windows APIs (conditional compilation) | Complex (compile-time) |

#### Stream B: Credential Store (scope 2.1-2.11)

| Module | Functions | Complexity |
| --- | --- | --- |
| `credentials/store.rs` | `load()`, `save()`, `write_credentials_file()`, credential capture (file), canonical save + mirror, dashboard atomic write | 3x Trivial, 3x Moderate |
| `credentials/validate.rs` | `validate_account()` (uses `AccountNum` newtype) | Trivial |
| `credentials/oauth.rs` | `refresh_token()` — HTTP POST to Anthropic, atomic write, field preservation | Complex |
| `credentials/keychain.rs` | `keychain_service()` (SHA256 NFC hash), `write_keychain()`, credential capture (keychain read) | 1x Complex, 2x Moderate |

#### Stream C: Account Identity (scope 3.1-3.16)

| Module | Functions | Complexity |
| --- | --- | --- |
| `accounts/identity.rs` | `which_account()`, markers, token matching | 2x Moderate, 4x Trivial |
| `accounts/snapshot.rs` | `snapshot_account()` with PID caching | Complex |
| `accounts/discovery.rs` | Anthropic, 3P, manual, combined discovery | 3x Moderate, 3x Trivial |
| `accounts/profiles.rs` | Profile save | Trivial |
| `accounts/types.rs` | `AccountInfo` struct | Trivial |

#### Stream D: Broker/Sync (scope 5.1-5.7)

| Module | Functions | Complexity |
| --- | --- | --- |
| `daemon/broker.rs` | `broker_check()` with per-account lock + fanout, recovery from dead RT, failure flags | 2x Complex, 2x Trivial |
| `daemon/fanout.rs` | `scan_config_dirs()`, `fan_out_credentials()` | 1x Trivial, 1x Moderate |
| `daemon/sync.rs` | `backsync()` with monotonicity guard, `pullsync()` with strict-newer check | 1x Complex, 1x Moderate |

### Dependencies

- Phase 0 complete (workspace compiles)
- Stream B depends on Stream A (atomic writes, file permissions)
- Stream C depends on Stream B (credential loading)
- Stream D depends on Streams B + C (credential refresh, account detection)

### Critical Path

**Platform locking (1.7-1.10) -> Credential store (2.1-2.11) -> Broker (5.1-5.2) -> Backsync (5.6)**

This is the longest dependency chain. The broker's per-account lock depends on platform locking. The broker's recovery path depends on credential store read/write. Backsync's monotonicity guard depends on both.

### Parallelization

Streams A and B can start simultaneously (B's platform dependencies are limited to `atomic_replace` and `secure_file`, which are among the first functions implemented in A). Stream C can begin as soon as `credentials/store.rs` is functional. Stream D begins when both B and C are complete.

### Test Strategy

**Unit tests** (run in CI, all platforms):
- Every function with a "Trivial" complexity rating has straightforward unit tests
- Credential load/save: valid JSON, missing file, corrupt file, permission verification
- Account validation: boundary values (0, 1, 999, 1000, negative, non-numeric, path traversal strings)
- Keychain service name: SHA256(NFC(path))[:8] verified against v1.x output for 5 known paths (this is the single most critical compatibility test)

**Integration tests** (run in CI where platform allows):
- File locking: two processes, one acquires lock, other gets `None` from try-lock
- Atomic replace: concurrent writers (10 threads, 100 writes each), no corruption
- Process detection: spawn child, verify PID found, kill child, verify PID not found
- Broker: mock HTTP server, verify per-account lock (10 concurrent tasks, exactly 1 refresh)
- Backsync: set up live-newer-than-canonical, verify canonical updated; set up live-older, verify no downgrade

**Parity tests** (run against v1.x test fixtures):
- Credential refresh: same request body format, same field preservation
- Statusline format: same input JSON produces identical output string
- Account identity: same config dir layout produces same `which_account()` result

### Security Considerations

- Fix C1 from security analysis: all credential writes use `tempfile::NamedTempFile::persist()`, never raw `File::create()`
- `AccessToken` and `RefreshToken` newtypes prevent accidental logging (masked `Display`)
- Keychain service name derivation must be byte-identical to CC's implementation (ADR-003)

### Exit Criteria

- [ ] `cargo test` passes on all platforms with >90% line coverage on `credentials/` and `platform/`
- [ ] Keychain service name for `/Users/test/.claude/accounts/config-1` matches v1.x output exactly
- [ ] Broker integration test: 10 concurrent tasks, exactly 1 HTTP refresh call made
- [ ] Backsync monotonicity: newer-live updates canonical, older-live does not
- [ ] Recovery test: dead canonical RT + live sibling with good RT -> promotion + successful refresh
- [ ] All credential writes verified atomic (kill -9 during write loop, no corruption)
- [ ] Windows file locking works (verified on Windows CI runner)

### Effort

**8 autonomous sessions** (Streams A-D, partially parallel).

- Stream A: 2 sessions (platform abstraction, including Windows FFI)
- Stream B: 2.5 sessions (credential store + OAuth refresh + keychain)
- Stream C: 2 sessions (account identity + discovery)
- Stream D: 3.5 sessions (broker + sync, the most complex logic)
- Overlap: ~2 sessions saved from parallelization of A+B and C starting early
- **Net: 8 sessions, ~2.5 days wall clock at 3 parallel sessions/day**

---

## Phase 2: CLI Parity

### Scope

Every `csq` command works against the Rust core library. No daemon required. The output of this phase is a `csq` binary that is a drop-in replacement for the v1.x bash+Python toolchain.

### Deliverables

| Module | Functions | Scope Ref |
| --- | --- | --- |
| `rotation/swap.rs` | `swap_to()` with verification + delayed check | 4.1 |
| `rotation/picker.rs` | `pick_best()`, `suggest()` | 4.2-4.3 |
| `rotation/auto.rs` | `auto_rotate()` | 4.4 |
| `quota/update.rs` | `update_quota()` with payload-hash cursor | 6.1 |
| `quota/state.rs` | `load_state()` | 6.2 |
| `quota/statusline.rs` | `statusline_str()` with stuck-swap and broker-failure indicators | 6.4 |
| `quota/format.rs` | `fmt_time()`, `fmt_tokens()` | 6.5, 12.6 |
| `cli/statusline.rs` | `csq statusline` — replaces `statusline-quota.sh` entirely | 12.1-12.7 |
| `cli/run.rs` | `csq run` — auto-resolution, broker call, credential copy, env stripping, exec | 11.1-11.9 |
| `session/isolation.rs` | Symlink shared artifacts, Windows junctions | 11.3 |
| `session/settings.rs` | Settings deep merge (default + overlay) | 11.4 |
| `session/setup.rs` | Onboarding flag, credential copy | 11.5, 11.7 |
| `providers/setkey.rs` | `csq setkey` — provider config with skeleton creation | 10.1 |
| `providers/catalog.rs` | Provider skeletons, model catalog | 10.2, 10.9 |
| `providers/validate.rs` | Key validation HTTP probe | 10.10 |
| `providers/repair.rs` | JSON auto-repair for truncated profiles | 10.11 |
| `providers/ollama.rs` | `ollama list` integration | 10.8 |
| `cli/keys.rs` | `csq listkeys`, `csq rmkey` | 10.3-10.4 |
| `cli/models.rs` | `csq models` (list all, list provider, switch) | 10.5-10.7 |
| `cli/status.rs` | `csq status` | 6.3 |
| `cli/login.rs` | `csq login` (browser flow via `claude auth login`) | 9.6 |
| `cli/install.rs` | `csq install` (self-installing), settings.json patching, migration cleanup | 14.1-14.2, 14.5 |
| `cli/update.rs` | `csq update`, `auto_update_bg()` with checksum | 14.3-14.4 |
| `main.rs` | `clap` routing, subcommands, numeric-first-arg detection | 15.1-15.4 |

### Dependencies

- Phase 1 complete (all core libraries functional)
- `rotation/swap.rs` depends on `credentials/` and `accounts/`
- `cli/run.rs` depends on everything above it in the dependency graph
- `cli/statusline.rs` depends on `quota/` and `accounts/`

### Critical Path

**swap_to() -> cli/run.rs -> cli/statusline.rs**

`csq run` is the most complex CLI command and depends on swap, broker, credential copy, settings merge, and symlink setup. The statusline is the second most complex because it replaces an entire bash script.

### Parallelization

Three parallel streams:

1. **Swap + Quota + Statusline**: swap_to() -> update_quota() -> statusline_str() -> csq statusline
2. **Session (run)**: isolation, settings merge, setup -> csq run (joins stream 1 at the end)
3. **Providers + Models + Install**: Fully independent of streams 1-2. Can proceed as soon as Phase 1's platform layer is available.

### Test Strategy

**End-to-end parity tests** (the primary validation):

For each CLI command, create a test fixture with:
- Input: the same config directory layout, credential files, and environment variables that v1.x would see
- Expected output: captured from v1.x running against that fixture
- Actual output: the v2.0 command running against the same fixture

Commands with parity tests:
- `csq status` — same format string
- `csq statusline` — same format given same CC JSON input
- `csq swap N` — same credential files written
- `csq run N` — same environment variables, same config dir structure
- `csq suggest` — same JSON output
- `csq listkeys` — same masked output

**Integration tests**:
- `csq run` with 0/1/2+ accounts — correct behavior for each case
- `csq swap` with delayed verification — verify the 2-second background check runs
- `csq install` — verify settings.json modified correctly, v1.x artifacts cleaned up
- Statusline with broker-failure flag — verify `LOGIN-NEEDED` prefix appears

### Exit Criteria

- [ ] Every `csq` subcommand from v1.x has a working v2.0 equivalent
- [ ] Parity tests pass for all commands with deterministic output
- [ ] `csq run 1` launches Claude Code with correct `CLAUDE_CONFIG_DIR` and environment
- [ ] `csq statusline` produces output within 50ms (vs 400ms v1.x baseline)
- [ ] `csq swap 2` completes within 20ms
- [ ] `csq install` correctly patches `settings.json` to use `csq statusline` instead of the bash script
- [ ] Binary size < 10MB (CLI-only, release build, stripped)
- [ ] Manual smoke test: run 3 concurrent CC sessions with different accounts, swap between them, verify statusline updates

### Effort

**8 autonomous sessions** (three parallel streams).

- Stream 1 (Swap + Quota + Statusline): 3.5 sessions
- Stream 2 (Session/Run): 2 sessions
- Stream 3 (Providers + Models + Install + Update): 3 sessions
- Overlap: ~1.5 sessions saved from full parallelization of stream 3
- Integration + parity testing: 1 session
- **Net: 8 sessions, ~3 days wall clock**

---

## Phase 3: Dashboard + System Tray

### Scope

The daemon, HTTP API, Svelte dashboard, and system tray. This is the v2.0 value proposition: what makes the rewrite worth doing. The daemon replaces v1.x's subprocess-per-statusline-render model with a single persistent process.

### Deliverables

#### Daemon Core

| Module | Functions | Scope Ref |
| --- | --- | --- |
| `daemon/lifecycle.rs` | Start, stop, health check, PID file, single-instance guard | New |
| `daemon/ipc.rs` | Unix socket (macOS/Linux) / named pipe (Windows) server, HTTP/1.1 over socket | New (ADR-005) |
| `daemon/refresher.rs` | Background token refresh (2-hour window, per-account async Mutex, fanout) | 8.1-8.5 (ADR-006) |
| `daemon/poller.rs` | Background usage polling (Anthropic + 3P), staggered start, exponential backoff | 7.1-7.4 |
| `daemon/cache.rs` | In-memory cache with TTL (`RwLock<HashMap>`) | 7.5 |
| `daemon/api.rs` | HTTP API routes: accounts, usage, tokens, refresh, login, callback | 13.1-13.11 |
| `daemon/mod.rs` | Server lifecycle, subsystem initialization, graceful shutdown | 13.10 |

#### CLI-to-Daemon Delegation

| Module | Change |
| --- | --- |
| `cli/status.rs` | Try daemon IPC first, fall back to direct file read |
| `cli/statusline.rs` | Try daemon IPC first, fall back to synchronous computation |
| `rotation/swap.rs` | Notify daemon after swap so it can update cache |
| `cli/run.rs` | Notify daemon of new session start |

#### OAuth (Dashboard)

| Module | Functions | Scope Ref |
| --- | --- | --- |
| `oauth/pkce.rs` | PKCE code verifier/challenge generation (RFC 7636) | 9.1, 9.4-9.5 |
| `oauth/callback.rs` | Handle OAuth callback, single-use state consumption | 9.2 |
| `oauth/exchange.rs` | Code-for-token exchange with Anthropic | 9.3 |

#### Desktop (Tauri)

| Module | Functions | Scope Ref |
| --- | --- | --- |
| Tauri IPC commands | Svelte -> Rust command handlers for all dashboard operations | New |
| System tray | Account status, quick-swap menu, open dashboard, quit | New |
| Dashboard views | Account list, usage bars, token health, login flow | New |
| Tauri security | IPC allowlist, CSP, isolation mode | New |

### Dependencies

- Phase 2 complete (CLI parity — all core logic works)
- Daemon depends on broker/sync (Phase 1) and quota/status (Phase 2)
- HTTP API depends on daemon core
- OAuth depends on HTTP API (callback handler)
- Dashboard UI depends on HTTP API (data source)
- System tray depends on daemon (status data)

### Critical Path

**Daemon lifecycle -> IPC server -> Token refresher + Usage poller -> HTTP API -> Dashboard UI**

The tray and dashboard UI are at the end of the chain. The OAuth flow depends on the HTTP API for the callback endpoint.

### Parallelization

Two major parallel streams:

1. **Daemon + API**: lifecycle -> IPC -> refresher + poller (parallel) -> cache -> HTTP API
2. **Frontend + Tray**: Tauri commands -> Svelte components -> System tray (can start as soon as the HTTP API contract is defined, using mock data, then integrate when the real API is ready)

The OAuth module can be built independently and integrated when the HTTP API is ready.

### Test Strategy

**Daemon integration tests**:
- Start daemon, verify PID file created, health endpoint responds
- Start second daemon, verify single-instance guard rejects it
- CLI command with daemon running: verify IPC delegation (response within 5ms)
- CLI command with daemon stopped: verify graceful fallback to direct mode
- Kill daemon (SIGKILL), verify CLI detects stale socket and falls back

**Token refresher tests**:
- Mock HTTP server with configurable response delays
- Verify 2-hour-ahead refresh window triggers correctly
- Verify per-account lock: 10 concurrent refresh attempts, exactly 1 HTTP call
- Verify fanout: after refresh, all matching config dirs have new credentials
- Verify recovery: dead canonical RT + live sibling -> promotion

**Usage poller tests**:
- Mock HTTP server returning usage JSON
- Verify staggered start (accounts don't all poll at the same instant)
- Verify exponential backoff on 429
- Verify 401 marks account as needing re-login

**Dashboard UI tests**:
- Svelte component tests (vitest + testing-library)
- Account list renders with mock data
- Usage bars update reactively when store changes
- OAuth login button initiates PKCE flow

**System tray tests** (manual):
- Tray icon appears on launch
- Menu shows all accounts with status
- Quick-swap changes active account
- "Open Dashboard" opens webview window

### Exit Criteria

- [ ] `csq daemon start` launches background process, PID file created
- [ ] `csq daemon stop` gracefully shuts down, PID file removed
- [ ] `csq status` with daemon running returns within 5ms (vs 30ms without daemon)
- [ ] Dashboard at `http://127.0.0.1:8420` shows all accounts with usage and token health
- [ ] OAuth login from dashboard stores credentials and daemon begins polling
- [ ] System tray shows account menu with current account highlighted
- [ ] Quick-swap from tray menu works (credential files updated, statusline reflects change)
- [ ] Token refresher: tokens refreshed 2 hours before expiry, fanout to all config dirs
- [ ] Daemon memory: < 30MB idle with 7 accounts
- [ ] All operations work without daemon (fallback to direct mode)

### Effort

**12 autonomous sessions** (two parallel streams).

- Stream 1 (Daemon + API): 6.5 sessions
  - Lifecycle + IPC: 2.5
  - Refresher: 1.5
  - Poller + Cache: 1.5
  - HTTP API: 1.5 (reuses existing route structure from v1.x)
  - CLI delegation: 0.5 (modification of existing commands)
- Stream 2 (Frontend + Tray): 5 sessions
  - Tauri commands + IPC allowlist: 1
  - Dashboard UI (account list, usage, tokens): 1.5
  - OAuth login flow UI: 1
  - System tray: 1
  - Integration testing: 0.5
- OAuth module: 1 session (parallel with both streams)
- **Net: 12 sessions, ~4 days wall clock**

---

## Phase 4: Cross-Platform + Packaging

### Scope

Production readiness: code signing, auto-update, platform-specific packaging, installation scripts, and v1.x migration tooling.

### Deliverables

| Deliverable | Details |
| --- | --- |
| macOS code signing | Apple Developer ID, notarization via `xcrun notarytool`, Gatekeeper-passing `.app` bundle |
| Windows code signing | Authenticode signing (or self-signed for initial release) |
| Tauri auto-update | Ed25519 signed update manifests, `tauri-plugin-updater`, update check on launch + daily interval |
| Homebrew tap | `terrene-foundation/tap` formula for macOS/Linux |
| Scoop manifest | `claude-squad.json` for Windows |
| .deb / .rpm packages | Built by Tauri's bundler, published to GitHub Releases |
| AppImage | Linux portable binary |
| Curl-pipe installer | `curl -sSL https://csq.terrene.dev/install | sh` — downloads binary, runs `csq install` |
| `csq doctor` | Diagnostic command: checks daemon status, credential health, Claude Code version, platform info |
| Shell completions | `csq completions bash/zsh/fish/powershell` via `clap_complete` |
| `--json` output | All commands support `--json` flag for machine-readable output |
| v1.x migration | `csq install` detects v1.x artifacts and migrates: removes old bash scripts, converts settings.json, preserves credentials |

### Dependencies

- Phase 3 complete (daemon + desktop functional)
- Code signing requires Apple Developer ID (external dependency — calendar-time gated)
- Homebrew tap requires a GitHub repo for the tap

### Critical Path

**Code signing -> Auto-update -> Homebrew/Scoop -> Curl installer**

Code signing must be done before auto-update can work (users need to trust the binary). The installer downloads a signed binary.

### Parallelization

Highly parallelizable. Code signing, packaging formats, CLI polish, and migration tooling are all independent:

1. **Signing + Update**: code signing -> auto-update manifest -> CI pipeline
2. **Packaging**: Homebrew, Scoop, .deb/.rpm (all independent)
3. **CLI polish**: `csq doctor`, shell completions, `--json` output (all independent)
4. **Migration**: v1.x detection + cleanup logic

### Test Strategy

- Auto-update: test binary downloads correct version, verifies Ed25519 signature, atomic replace succeeds
- `csq doctor`: verify output on clean system, system with daemon, system with broken credentials
- Shell completions: verify `csq <TAB>` completes subcommands on bash/zsh
- Migration: set up v1.x file layout, run `csq install`, verify cleanup and settings conversion
- Cross-platform: manual smoke test on macOS (arm64), Linux (x86_64 VM), Windows (x86_64 VM)

### Exit Criteria

- [ ] Signed macOS .app passes Gatekeeper (`spctl --assess`)
- [ ] `csq update` downloads and installs new version, old version replaced atomically
- [ ] `brew install terrene-foundation/tap/csq` works on macOS
- [ ] `scoop install csq` works on Windows
- [ ] Curl-pipe installer works on fresh macOS and Linux systems
- [ ] `csq doctor` reports all-green on a correctly configured system
- [ ] `csq completions zsh | source /dev/stdin` enables tab completion
- [ ] `csq status --json` outputs valid JSON
- [ ] v1.x migration: `csq install` on a system with v1.x csq results in working v2.0 with preserved credentials

### Effort

**5 autonomous sessions** (highly parallel).

- Signing + Update: 1.5 sessions
- Packaging: 1.5 sessions (parallel)
- CLI polish: 1.5 sessions (parallel)
- Migration: 0.5 sessions
- Cross-platform smoke testing: 0.5 sessions
- **Net: 5 sessions, ~2 days wall clock**

---

## Effort Summary

| Phase | Sessions | Wall Clock | Parallelizable With |
| --- | :---: | :---: | --- |
| Phase 0: Scaffolding | 1 | 0.5 days | -- |
| Phase 1: Core Credentials | 8 | 2.5 days | -- |
| Phase 2: CLI Parity | 8 | 3 days | Phase 3 Stream 2 (frontend) |
| Phase 3: Dashboard + Tray | 12 | 4 days | Phase 2 Stream 3 (providers) |
| Phase 4: Cross-Platform | 5 | 2 days | -- |
| **Total** | **34** | **~12 days** | |

Wall clock assumes 3-4 parallel autonomous sessions per day with mature COC institutional knowledge.

### Phase Overlap Opportunities

**Phase 2 + Phase 3 overlap**: Phase 3's frontend stream (Svelte components, system tray) can begin as soon as the HTTP API contract is defined — which happens during Phase 3 daemon work. But the Tauri scaffolding from Phase 0 is already available. A frontend developer (or agent) can start building UI components with mock data during Phase 2, then wire up real data when the daemon API is ready in Phase 3.

**Phase 2 Stream 3 + Phase 3**: Provider management, model commands, and install/update are completely independent of the daemon. They can proceed during Phase 3 if Phase 2 runs behind schedule.

With maximum overlap, the critical path is: Phase 0 (0.5d) -> Phase 1 (2.5d) -> Phase 2+3 critical path (5d) -> Phase 4 (2d) = **~10 days**.

---

## Migration Strategy

### User Journey: v1.x to v2.0

**Goal**: Zero credential loss, zero downtime, one command.

#### Step 1: Install v2.0

```
curl -sSL https://csq.terrene.dev/install | sh
```

The installer:
1. Downloads the platform-appropriate `csq` binary
2. Places it in `~/.local/bin/csq` (or `~/.claude/bin/csq`), replacing the v1.x bash script
3. Runs `csq install` automatically

#### Step 2: `csq install` Migration Logic

`csq install` detects v1.x artifacts and migrates:

1. **Credential files preserved**: `~/.claude/accounts/credentials/*.json` are the same format in v1.x and v2.0. No conversion needed. The Rust code reads the same JSON schema.

2. **Profiles preserved**: `~/.claude/accounts/profiles.json` is the same format. No conversion needed.

3. **Quota state preserved**: `~/.claude/accounts/quota.json` is the same format. No conversion needed.

4. **Settings.json updated**: The Claude Code `settings.json` is patched to replace:
   - v1.x: `"command": "bash /path/to/statusline-quota.sh"`
   - v2.0: `"command": "csq statusline"`
   This is the only configuration change visible to the user.

5. **Dead artifacts removed**:
   - `statusline-command.sh` (v1.x wrapper) — deleted
   - `statusline-quota.sh` symlink/file — deleted if in the accounts directory
   - `rotate.md` — deleted
   - `auto-rotate-hook.sh` — deleted (was already a no-op)

6. **Python dependency dropped**: v2.0 has no Python dependency. The install script does NOT remove Python (the user may need it for other things), but it stops requiring it.

7. **Dashboard migration**: v1.x `python -m dashboard` is replaced by `csq daemon start` (background) or `csq app` (desktop). The old dashboard Python code is not deleted (it lives in the git repo, not in the install directory), but it is no longer referenced.

#### Step 3: Verify

```
csq doctor
```

Outputs:
- Binary version: v2.0.0
- Daemon: not running (start with `csq daemon start`)
- Accounts: 7 configured (all credentials valid)
- Claude Code: v1.x.x detected at /usr/local/bin/claude
- Settings: statusline command updated to csq v2.0
- Platform: macOS arm64

#### Step 4: Start Daemon (Optional)

```
csq daemon start
```

Or launch the desktop app (`csq app`) which starts the daemon automatically.

### Backwards Compatibility Guarantees

1. **All v1.x CLI commands work identically**: `csq run`, `csq swap`, `csq status`, `csq login`, `csq setkey`, `csq models`, `csq listkeys`, `csq rmkey`, `csq suggest`, `csq update`. Same arguments, same output format.

2. **Credential files are the same format**: v2.0 reads v1.x credentials. v1.x can read v2.0 credentials (if a user needs to roll back). The JSON schema is unchanged.

3. **Statusline format is identical**: The RPROMPT string produced by `csq statusline` matches `statusline-quota.sh` exactly for the same input data. Users who have customized their shell prompts see no change.

4. **Environment variables preserved**: `CLAUDE_CONFIG_DIR`, `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL` — all behave identically.

5. **No forced daemon**: The daemon is optional. All CLI commands work without it (falling back to direct file access, like v1.x). The daemon adds speed and coordination, but is never required.

### Rollback

If v2.0 has a critical bug:

```
# Re-install v1.x
curl -sSL https://raw.githubusercontent.com/terrene-foundation/claude-squad/v1.x/install.sh | bash
```

Credential files are compatible in both directions. The only thing that needs to change is the statusline command in `settings.json` (the v1.x installer handles this).

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Keychain service name mismatch with CC | Low | Critical | Parity test with 5 known paths. Byte-level comparison. Block Phase 1 exit on this test. |
| Windows file locking behavior differs | Medium | High | Dedicated Windows CI runner. Named mutex vs POSIX flock — different semantics. Tested in Phase 1. |
| Tauri auto-update requires code signing first | High | Medium | Code signing is an external dependency (Apple Developer ID). Schedule it early in Phase 4. Ship without auto-update if signing is delayed. |
| CC changes its internal refresh behavior | Low | High | Monitor CC releases. The 2-hour refresh window (ADR-006) provides a large buffer. Recovery path handles CC-wins-race scenario. |
| Binary size exceeds 25MB | Low | Low | Monitor with `cargo bloat`. Feature flags ensure CLI-only builds exclude Tauri. Strip symbols in release. |
| WebKitGTK missing on headless Linux | Medium | Low | CLI-only mode is fully functional. Desktop features gracefully degrade. Documented in README. |
| v1.x users have customized statusline-quota.sh | Low | Medium | `csq install` checks for modifications before deleting. If modified, warn and preserve as `.bak`. |

---

## Definition of Done

v2.0 is ready for release when:

1. All Phase 0-3 exit criteria are met
2. Phase 4 packaging produces signed binaries for macOS, Linux, and Windows
3. v1.x migration works end-to-end (tested on fresh system with v1.x installed)
4. `csq doctor` reports all-green on all three platforms
5. Manual smoke test: 7 accounts, 5 concurrent sessions, swap/rotate/refresh all working
6. README updated with v2.0 installation instructions
7. GitHub Release published with binaries and changelog
