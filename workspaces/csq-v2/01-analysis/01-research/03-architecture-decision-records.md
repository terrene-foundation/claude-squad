# csq v2.0 — Architecture Decision Records

---

## ADR-001: Rust + Tauri for the Rewrite

### Status

Proposed

### Context

csq v1.x is bash + Python (stdlib-only). It works on macOS, Linux, WSL, and Windows (Git Bash), but has structural limitations:

1. **Three Python subprocesses per statusline render** (~400ms). With 15 terminals, that is 45 Python interpreter starts every few seconds.
2. **No long-running daemon**. The broker pattern (per-account try-lock on every statusline render) was a workaround for not having a persistent process. It works but is fragile and burns CPU.
3. **No desktop app**. The dashboard is a separate `python -m dashboard` that the user must start manually.
4. **Installation friction**. Requires Python 3, jq, curl. Windows needs Git Bash. Users hit PATH issues, Python version conflicts, and missing dependencies.
5. **No system tray**. Users must remember to check the dashboard or read the statusline.

The rewrite target must provide: compiled single binary (no runtime), desktop app with system tray, cross-platform support (macOS/Linux/Windows natively), and a web-based UI for the dashboard.

### Decision

**Rust for the core + Tauri for the desktop app.**

### Consequences

#### Positive

- **Single binary**: `csq` CLI is a standalone Rust binary with zero runtime dependencies. Download and run.
- **Tauri desktop**: System tray + webview dashboard. Uses the OS's native webview (WebKit on macOS, WebKitGTK on Linux, WebView2 on Windows) so the app is 5-15MB, not 150MB+ like Electron.
- **Performance**: Rust eliminates the Python interpreter tax. Statusline queries become IPC calls to the daemon (<5ms vs 400ms).
- **Memory**: Idle daemon at <30MB (Tauri webview baseline) vs 50MB+ for Python HTTP server + threads.
- **Cross-compilation**: `cross` and `cargo-tauri` support macOS (arm64/x86_64), Linux (x86_64/arm64), and Windows (x86_64) from CI.
- **Type safety**: The credential management code is the most bug-prone area (see journal entries 0005, 0011, 0014, 0017, 0019). Rust's type system and ownership model prevent entire classes of bugs: use-after-free on credential data, data races on shared state, null-pointer dereferences on missing files.

#### Negative

- **Build complexity**: Tauri requires Node.js for the frontend build step (Svelte). The final binary has no Node dependency, but the build pipeline does.
- **WebKitGTK dependency on Linux**: Users on headless Linux servers cannot use the desktop app. CLI-only mode must be fully functional.
- **Learning curve**: Contributors familiar with Python/bash will need Rust knowledge. Mitigated by clean module boundaries and thorough documentation.
- **Longer initial development**: Rust is more verbose than Python for IO-heavy code. Estimated 2-3 autonomous sessions vs 1 session if we stayed with Python.

### Alternatives Considered

#### Go + Wails

- **Pros**: Fast compilation, simpler concurrency model (goroutines), Wails provides webview.
- **Cons**: Wails is less mature than Tauri (smaller community, fewer plugins). Go's error handling is verbose without sum types. No equivalent to Rust's `keyring` crate ecosystem. GC pauses, while tiny, are non-zero — Rust has none.
- **Rejected**: Tauri's maturity and plugin ecosystem (updater, system tray, deep links) are decisive.

#### Electron

- **Pros**: Mature, well-known, huge ecosystem, easy to find contributors.
- **Cons**: 150MB+ binary (bundles Chromium). 100MB+ RAM idle. Contradicts the "single binary, zero dependencies" goal. Foundation independence rules prohibit unnecessary third-party dependencies (Chromium is a Google project).
- **Rejected**: Binary size and memory usage are disqualifying.

#### Stay with Python (add daemon)

- **Pros**: Zero migration risk. Familiar codebase. Could add a daemon via `multiprocessing` or `asyncio`.
- **Cons**: Still requires Python 3 installed. Still requires jq. Still no system tray without PyQt/Tkinter (adding the dependencies the project explicitly forbids). Still ~30MB per CLI invocation.
- **Rejected**: The vision explicitly calls for "one download, one binary, zero runtime dependencies."

#### Python + PyInstaller/Nuitka

- **Pros**: Keep Python codebase, compile to binary.
- **Cons**: PyInstaller bundles produce 50-100MB binaries. Nuitka compilation is slow and produces 30-50MB binaries. Neither provides a system tray or webview. Still need a separate framework for the desktop app.
- **Rejected**: Worse binary size than Rust, no desktop app integration.

---

## ADR-002: Svelte for the Frontend

### Status

Proposed

### Context

Tauri supports any web frontend framework (or plain HTML/CSS/JS). The dashboard needs:

- Account list with real-time usage updates
- Token health indicators with auto-refresh
- OAuth login flow integration
- Model management UI
- Settings/profile editor

The frontend bundle size matters because it is embedded in the binary.

### Decision

**Svelte (SvelteKit in SPA mode).**

### Consequences

#### Positive

- **Bundle size**: Svelte compiles to vanilla JS with no runtime. Typical dashboard app: 30-50KB gzipped. React equivalent: 100-150KB.
- **Tauri integration**: `@tauri-apps/api` works identically with Svelte and React. No framework-specific limitations.
- **Reactivity model**: Svelte's reactive declarations (`$:`) map naturally to the dashboard's "poll and display" pattern. No `useEffect` dependency array bugs.
- **Simplicity**: The dashboard is a single-page app with 5-6 views. Svelte's component model is simpler than React for this scale.
- **Tauri template**: `create-tauri-app` has first-class Svelte template. Zero configuration.

#### Negative

- **Smaller ecosystem**: Fewer UI component libraries than React. Mitigated: the dashboard is custom-designed, not using a component library.
- **Fewer contributors**: React developers outnumber Svelte developers ~10:1. Mitigated: the frontend is small (<2000 lines estimated) and Svelte syntax is learnable in hours by any JS developer.

### Alternatives Considered

#### React

- **Pros**: Largest ecosystem, most contributors, most Tauri examples.
- **Cons**: 40KB runtime overhead before any app code. `useEffect` is a footgun for real-time polling. Overkill for a dashboard with 5 views.
- **Rejected**: Bundle size penalty and complexity overhead not justified for this app size.

#### Vue

- **Pros**: Good middle ground, strong Tauri support, smaller than React.
- **Cons**: Options API vs Composition API split creates confusion. Template syntax is more verbose than Svelte for reactive patterns.
- **Rejected**: No clear advantage over Svelte for this use case.

#### Solid.js

- **Pros**: React-like but compiled (no virtual DOM), excellent performance.
- **Cons**: Smaller ecosystem than Svelte. Less Tauri community adoption.
- **Rejected**: Svelte has better Tauri integration and community support.

#### Plain HTML/CSS/JS (like v1.x)

- **Pros**: Zero build step. Maximum simplicity.
- **Cons**: No component model makes the UI unmaintainable as features grow. No reactive updates without manual DOM manipulation. The v1.x dashboard JS is already awkward.
- **Rejected**: The v2.0 dashboard needs real-time updates, multiple views, and interactive controls. A framework pays for itself.

---

## ADR-003: Credential Storage — keyring Crate + File Fallback

### Status

Proposed

### Context

v1.x uses two credential storage paths:

1. **Files** (`credentials/N.json`): Primary store on all platforms. Atomic writes, `0o600` permissions.
2. **macOS Keychain**: Secondary store via `security` CLI subprocess. Used by CC as a fallback. 3-second timeout to avoid hangs under contention.

The `security` CLI approach has problems:

- Subprocess spawning is slow (50-100ms per call)
- 3-second timeout is both too long (blocks the statusline) and too short (fails under heavy contention with 15 terminals)
- Hex encoding/decoding of JSON is fragile
- No equivalent on Linux or Windows without different tools (`secret-tool`, `CredWrite`)

### Decision

**Use the `keyring` crate for cross-platform keychain access, with file-based storage as the primary store and keychain as a best-effort secondary.**

The `keyring` crate provides a unified API across:

- macOS: Security.framework (same keychain as the `security` CLI)
- Linux: `libsecret` (GNOME Keyring / KDE Wallet)
- Windows: Windows Credential Manager

### Consequences

#### Positive

- **Cross-platform keychain**: Linux and Windows get keychain support that v1.x lacks. Token recovery is possible even if credential files are deleted.
- **No subprocess spawning**: Direct FFI/library calls are 100x faster than `security` CLI.
- **No hex encoding**: The crate handles serialization. Store JSON directly.
- **No timeout hacks**: Library calls don't hang like the `security` CLI under contention.

#### Negative

- **Native library dependency**: On Linux, requires `libsecret` (usually pre-installed on desktop distros, absent on servers). CLI-only mode on headless Linux falls back to file-only.
- **Keychain service name must match CC**: CC derives its keychain service name as `Claude Code-credentials-{sha256(NFC(dir))[:8]}`. The Rust code must produce identical service names. This is a correctness-critical compatibility requirement.
- **Keychain remains secondary**: Files are still the primary store (CC reads `.credentials.json` first). Keychain is a fallback for CC's internal token refresh path. The file store is what csq controls; the keychain is a courtesy.

### Alternatives Considered

#### File-only (drop keychain entirely)

- **Pros**: Simplest. No native library dependencies. Files are the primary store anyway.
- **Cons**: CC's internal refresh path on macOS reads the keychain as a fallback. If csq doesn't write to the keychain, CC may use a stale keychain entry after csq swaps accounts. This was observed in v1.x (journal/0021) and is the reason keychain writes exist.
- **Rejected**: Dropping keychain causes subtle CC misbehavior on macOS.

#### Direct Security.framework FFI (macOS only)

- **Pros**: No crate dependency. Full control over keychain operations.
- **Cons**: macOS-only. Would need separate implementations for Linux and Windows. The `keyring` crate already does this correctly.
- **Rejected**: Reinventing cross-platform keychain access that a well-maintained crate provides.

---

## ADR-004: Single Binary — CLI and Daemon in One

### Status

Proposed

### Context

v1.x has three separate programs:

1. `csq` (bash) — CLI entry point
2. `rotation-engine.py` — core logic
3. `dashboard/` — HTTP server + background tasks

v2.0 must decide: one binary or multiple?

### Decision

**Single binary with subcommand routing.** The `csq` binary operates in three modes:

1. **CLI mode** (default): `csq run`, `csq swap`, `csq status`, etc. Short-lived process. If a daemon is running, delegates to it via IPC. If not, operates directly on files (fallback).
2. **Daemon mode**: `csq daemon start`. Long-running process. Runs token refresh, usage polling, broker logic. Exposes HTTP API for CLI and dashboard.
3. **Desktop mode**: `csq app` or launched from Applications. Starts daemon + opens Tauri window with system tray.

All three modes are the same binary. No separate installation for CLI vs desktop.

### Consequences

#### Positive

- **Single download**: Users install one file. `csq` works as CLI immediately. `csq app` launches the desktop experience.
- **No version skew**: CLI and daemon are always the same version. No "daemon is v2.1 but CLI is v2.0" bugs.
- **Shared code**: Credential management, token refresh, account discovery are compiled once and shared across all modes.
- **Simpler distribution**: One binary per platform. One Homebrew formula. One Scoop manifest.

#### Negative

- **Larger CLI binary**: Including Tauri/webview linkage in the CLI binary adds size. Mitigated: Tauri uses the system webview (no Chromium bundled), so the overhead is ~2-3MB of Tauri framework code.
- **Conditional compilation complexity**: Some code (system tray, webview initialization) is desktop-only. Use Cargo features: `default = ["cli"]`, `desktop = ["tauri"]`. CLI-only builds omit Tauri entirely.

### Alternatives Considered

#### Separate CLI + Desktop App

- **Pros**: CLI binary is minimal (<5MB). Desktop app is separate download.
- **Cons**: Two binaries to keep in sync. Users must install both. Version skew bugs. Two update channels.
- **Rejected**: Complexity outweighs the 3-5MB binary size savings.

#### CLI + Daemon as Separate Binaries

- **Pros**: `csq` is tiny; `csqd` is the daemon.
- **Cons**: Users must manage two processes. "Is the daemon running?" becomes a support question. Auto-start logic is more complex with two binaries.
- **Rejected**: Single binary with `csq daemon start` is simpler for users.

---

## ADR-005: CLI-to-Daemon Communication — Unix Socket + HTTP

### Status

Proposed

### Context

When the daemon is running, CLI commands should delegate to it instead of reading files directly. This avoids file-locking contention and provides real-time data (cached usage, token health) without disk IO.

The communication channel needs to:

- Work on macOS, Linux, and Windows
- Be fast (<5ms round-trip for statusline queries)
- Support structured data (JSON)
- Not require authentication (local-only)
- Not conflict with the dashboard's HTTP server

### Decision

**Unix domain socket on macOS/Linux. Named pipe on Windows. HTTP protocol over the socket.**

The daemon listens on:

- macOS/Linux: `$XDG_RUNTIME_DIR/csq.sock` or `/tmp/csq-{uid}.sock`
- Windows: `\\.\pipe\csq-{username}`

The protocol is HTTP/1.1 over the socket. The CLI uses `hyper` (or `reqwest` with Unix socket support) to make requests. The dashboard HTTP server (for the web UI) runs on `127.0.0.1:8420` as in v1.x, proxying to the same daemon logic.

### Consequences

#### Positive

- **Fast**: Unix socket round-trip is <1ms. Named pipe on Windows is comparable.
- **Standard protocol**: HTTP over Unix socket is well-supported by Rust HTTP libraries. JSON request/response bodies.
- **No port conflicts**: Unix sockets don't consume TCP ports. Multiple csq installations (different users) don't conflict.
- **Dashboard reuse**: The daemon's HTTP handlers serve both the CLI (via socket) and the dashboard (via TCP). Same code path.
- **Tooling**: `curl --unix-socket /tmp/csq.sock http://localhost/api/status` for debugging.

#### Negative

- **Windows named pipes differ from Unix sockets**: The `hyper` ecosystem has less mature named pipe support. May need `tokio::net::windows::named_pipe` with a custom `hyper` connector.
- **Socket file cleanup**: If the daemon crashes, the socket file may persist. Must check liveness (connect attempt) before assuming the daemon is running.

### Alternatives Considered

#### TCP HTTP (localhost:port)

- **Pros**: Works identically on all platforms. Simple.
- **Cons**: Port conflicts (another process on 8420). Firewall warnings on some systems. Slightly slower than Unix socket (~1ms vs <0.5ms). Security concern: any local process can connect (same as Unix socket, but more visible to port scanners).
- **Rejected**: Port conflicts are a real user-facing problem. Unix socket eliminates them.

#### gRPC

- **Pros**: Strong typing, code generation, streaming.
- **Cons**: Adds `tonic` + `prost` dependencies (~2MB binary size). Overkill for <20 RPC methods. gRPC tooling is heavier than `curl` for debugging.
- **Rejected**: HTTP+JSON is sufficient and simpler.

#### Plain TCP with custom protocol

- **Pros**: Minimal dependencies.
- **Cons**: Reinventing HTTP. No ecosystem tooling for debugging. Must handle framing, content-length, error codes manually.
- **Rejected**: HTTP is a solved problem.

#### Shared memory / memory-mapped files

- **Pros**: Fastest possible IPC. Zero serialization for fixed-size data.
- **Cons**: Complex synchronization. No request/response pattern. Fragile across crashes.
- **Rejected**: The data is small JSON payloads; serialization cost is negligible.

---

## ADR-006: Single-Use Refresh Token Race Condition

### Status

Proposed

### Context

Anthropic's OAuth implementation sometimes rotates the refresh token on each use: the response to a token refresh includes a new refresh token that invalidates the previous one. This creates a race condition when multiple processes hold the same refresh token:

1. Terminal A and Terminal B both have refresh token RT1.
2. Terminal A refreshes: gets AT2 + RT2. RT1 is now invalid.
3. Terminal B refreshes with RT1: gets 401. B is now stuck.

v1.x solved this with the **broker pattern** (journal/0018): a single refresher per account, per-account try-lock, and fanout to all config dirs. The broker also has a **recovery path** (journal/0019): when the canonical RT is dead, it tries each live sibling's RT in turn.

v2.0 replaces the broker with a daemon that is the sole refresher. But the race condition between the daemon and CC's own internal refresh path remains.

### Decision

**The daemon is the sole external refresher. CC's internal refresh is suppressed by keeping tokens fresh ahead of expiry (2-hour refresh window). The recovery path is preserved for the residual race.**

Specifically:

1. **Daemon refresh window**: 2 hours before expiry (matching v1.x `REFRESH_AHEAD_SECS = 7200`). Anthropic tokens last ~5 hours. Refreshing at the 3-hour mark means CC never sees an expired token and never triggers its own refresh path.

2. **Per-account async lock**: `tokio::sync::Mutex<()>` per account. Only one refresh in flight per account at any time. Non-blocking try-lock for polling cycles; blocking lock for explicit refresh requests.

3. **Fanout after refresh**: After successful refresh, the daemon writes the new credentials to all `config-X/.credentials.json` files where `.csq-account` matches. CC's mtime check picks up the new file on its next API call.

4. **Recovery from CC race**: If CC's internal refresh wins the race (CC sees a near-expiry token before the daemon refreshes, and CC's refresh rotates the RT), the daemon's next refresh attempt will 401. Recovery: scan all `config-X/.credentials.json` for a live RT that differs from canonical, promote it, retry.

5. **LOGIN-NEEDED signaling**: If both the primary and recovery paths fail, the daemon marks the account as needing re-login and surfaces a notification (system tray alert + statusline indicator).

### Consequences

#### Positive

- **Eliminates the broker's process-spawning overhead**: No more try-lock via file system on every statusline render. The daemon holds the lock in memory.
- **Wider refresh window**: 2-hour window means CC almost never attempts its own refresh, reducing the race probability to near-zero.
- **Instant fanout**: Daemon writes to all config dirs immediately after refresh. No waiting for each terminal's next pullsync cycle.
- **Recovery is identical to v1.x**: The `_broker_recover_from_live()` algorithm is preserved, just implemented in Rust with async IO.

#### Negative

- **Daemon must be running for proactive refresh**: If the daemon is not running, terminals fall back to CC's internal refresh, which reintroduces the race. Mitigated: CLI fallback mode runs a synchronous broker check (like v1.x `csq run` does) when the daemon is unreachable.
- **Recovery path is complex**: Scanning config dirs, trying each sibling's RT, restoring on failure. This is inherent complexity of the problem, not the solution.

### Alternatives Considered

#### Per-slot OAuth sessions (Option B from journal/0018)

- **Pros**: Each terminal has its own independent OAuth session. No shared refresh tokens. No race.
- **Cons**: Requires K browser logins per account (where K = number of concurrent terminals). User explicitly rejected this as "stupid UX."
- **Rejected**: Same reason as v1.x.

#### Let CC handle all refreshes (remove csq refresh entirely)

- **Pros**: Simplest. Zero race conditions from csq's side.
- **Cons**: CC's internal refresh doesn't coordinate across config dirs. When CC in config-1 refreshes, config-2 still has the old RT. If Anthropic rotated the RT, config-2 is dead. This is the exact problem that created the broker.
- **Rejected**: Multi-terminal-same-account requires coordinated refresh.

#### Distributed lock via Anthropic's API

- **Pros**: Server-side coordination. No local races.
- **Cons**: Anthropic doesn't provide such an API. We can't modify the upstream.
- **Rejected**: Not possible.
