# 04 csq Daemon Architecture

Spec version: 1.0.0 | Status: DRAFT | Governs: daemon subsystems, IPC surface, refresh logic, sweep, supervisor

---

## 4.0 Scope

This spec defines csq's long-running daemon: what subsystems it hosts, how they interact, what the external contract looks like (Unix socket API), and the invariants that the daemon enforces regardless of which CLI or desktop process is talking to it.

## 4.1 Process model

The daemon is a tokio runtime embedded in one of two process hosts:

- **Desktop app**: `csq-desktop` starts the daemon in-process on launch. This is the canonical path on macOS for users running the Tauri UI.
- **Standalone**: `csq daemon start` launches the daemon as a detached process. Used on headless Linux servers and for debugging.

Only one daemon runs per user at a time. The Unix socket at `$base_dir/csq.sock` serves as both IPC and lockfile: binding the socket fails if another daemon is already running. The first process to bind wins; losers check socket liveness and either defer or take over if the existing daemon is dead.

See `csq-desktop/src-tauri/src/daemon_supervisor.rs` for the takeover/defer state machine.

## 4.2 Subsystems

### 4.2.1 Refresher (`daemon::refresher`)

**Responsibility:** keep each account's OAuth tokens fresh ahead of expiry.

**Cadence:** scans every 5 minutes. For each account:

1. Read canonical credentials from `credentials/<N>.json`.
2. If `expiresAt - now < REFRESH_AHEAD_SECS` (default 7200 = 2 hours), refresh.
3. Acquire per-account async lock (`tokio::sync::Mutex`).
4. POST to Anthropic's token endpoint with the refresh token.
5. On success: atomically write new tokens to BOTH `credentials/<N>.json` AND `config-<N>/.credentials.json`. Preserve `subscription_type` and `rate_limit_tier` from the existing file (subscription contamination guard — see `rules/account-terminal-separation.md` rule 4).
6. On 401: mark account LOGIN-NEEDED, surface via daemon API.
7. On 429: exponential backoff, capped at 40 minutes.

**Handle-dir interaction:** the refresher writes `config-<N>/.credentials.json` exactly once per refresh. Every handle dir whose `.credentials.json` symlink points at `config-<N>/` automatically sees the new content on its next `fs.stat`. This replaces the old `broker::fanout::fan_out_credentials` which iterated `config-*` dirs and copied credentials per-match — obsolete under the handle-dir model because there is exactly ONE permanent dir per account.

**Invariants:**

- Only one refresh in flight per account (per-account mutex).
- Writes are atomic (temp file + rename) with `0o600` permissions.
- Subscription metadata preserved on every write.

### 4.2.2 Usage poller (`daemon::usage_poller`)

**Responsibility:** poll Anthropic's `/api/oauth/usage` per account; poll third-party provider endpoints. Write to `quota.json`. Governed in detail by spec 05.

**Cadence:** Anthropic every 5 minutes; 3P every 15 minutes.

**Critical invariant:** the usage poller is the SOLE writer of `quota.json`. No CLI path, no statusline, no terminal-side code writes quota. Terminal-side quota attribution is unreliable (see `rules/account-terminal-separation.md` rule 1).

**Hang protection (gap from 2026-04-12):** the poller's main loop MUST wrap each `spawn_blocking` HTTP call in `tokio::time::timeout(30s, ...)` and MUST be run under a supervisor that respawns the task on panic with logged backtrace. The root cause of the poller stopping silently after 12:17 UTC on 2026-04-12 was an unguarded hang (likely in `tick_3p`) that blocked the loop forever. See journal `0031` related discussion and the fix plan in the implementation milestones.

### 4.2.3 Handle dir sweeper (`daemon::sweep`)

**Responsibility:** remove orphan `term-<pid>/` handle dirs whose owning `claude` process has exited.

**Cadence:** every 30 seconds.

**Actions:**

1. `readdir(accounts/)` and filter to entries matching `term-[0-9]+/`.
2. For each, read `.live-pid`.
3. Check liveness: Unix `kill(pid, 0)` — returns ESRCH if dead; Windows `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, pid)` — returns null if dead.
4. If dead: remove the handle dir (idempotent — ENOENT OK).
5. If alive: skip.

**Invariants:**

- A handle dir whose `.live-pid` file is missing is immediately swept (treated as malformed).
- A handle dir whose PID is alive is NEVER swept, even if its symlinks are broken or stale.
- The sweep is safe to run concurrently with `csq run` creating new handle dirs (atomic `create_dir` on the new one; the sweep only removes, never modifies existing dirs).

### 4.2.4 OAuth callback listener (`daemon::oauth_callback`)

**Responsibility:** serve the single localhost TCP route at `127.0.0.1:8420/oauth/callback` used by the browser OAuth flow. Authenticated by CSPRNG state token.

**Scope:** exactly one route; everything else belongs on the Unix socket. See `rules/security.md` rule 3 (MUST NOT / No Secrets on TCP Routes).

### 4.2.5 IPC server (`daemon::server`)

**Responsibility:** serve the Unix socket at `$base_dir/csq.sock`. HTTP/1.1 protocol, JSON bodies. Listed routes:

| Route                        | Purpose                                                           | Authentication    |
| ---------------------------- | ----------------------------------------------------------------- | ----------------- |
| `GET /api/health`            | Liveness check                                                    | None              |
| `GET /api/accounts`          | List accounts + refresh status + subscription tier                | None (local only) |
| `GET /api/usage`             | Return current `quota.json` snapshot                              | None              |
| `GET /api/refresh-status`    | Per-account refresh state                                         | None              |
| `POST /api/provision`        | Signal that account N was just logged in; start refresh + polling | None              |
| `POST /api/invalidate-cache` | Clear in-memory caches (e.g. after a swap)                        | None              |
| `POST /api/swap-report`      | Record a swap event for telemetry                                 | None              |

**Security layers (three-layer, see `rules/security.md` rule 7):**

1. **Socket file permissions**: umask `0o077` before `bind`, then explicit `chmod 0o600`.
2. **Peer credential verification**: `SO_PEERCRED` (Linux) / `LOCAL_PEERCRED` (macOS) rejects different-UID connections.
3. **Per-user socket directory**: `$XDG_RUNTIME_DIR` or `~/.claude/accounts/`.

## 4.3 Supervisor

The daemon supervisor (`csq-desktop/src-tauri/src/daemon_supervisor.rs`) handles:

- Takeover from stale daemons (PID dead but socket file present).
- Graceful shutdown on app quit.
- **Subsystem-level panic recovery:** each subsystem runs in a `tokio::spawn`ed task; the supervisor holds JoinHandles and on panic respawns with exponential backoff. Logs backtraces via the `tracing` subsystem.

## 4.4 Shutdown

On receipt of SIGTERM (standalone) or desktop app quit (embedded):

1. Supervisor signals `CancellationToken` to every subsystem.
2. Each subsystem drains in-flight work (refresh, poll, sweep) with a 5-second deadline.
3. Supervisor releases the PidFile and unbinds the Unix socket.
4. Process exits.

## 4.5 Cross-references

- `specs/01-cc-credential-architecture.md` section 1.7 — CC's `saveOAuthTokensIfNeeded` write path; the daemon refresher mirrors its subscription-preservation behavior.
- `specs/02-csq-handle-dir-model.md` section 2.5 — handle dir sweep invariants.
- `specs/05-quota-polling-contracts.md` — usage poller endpoint contracts.
- `rules/security.md` — daemon three-layer security, OAuth dual-listener rules.
- Journal `0006-DECISION-daemon-three-layer-security.md`, `0008-DECISION-cache-ownership-daemon-level.md`, `0011-DECISION-oauth-dual-listener-security.md`, `0026-DECISION-in-process-daemon-autostart-iterm-tagging-per-slot-3p-quota.md`.

## Revisions

- 2026-04-12 — 1.0.0 — Initial draft. Refresher no longer does per-handle-dir fanout; handle dir sweeper added; hang protection noted in usage poller.
