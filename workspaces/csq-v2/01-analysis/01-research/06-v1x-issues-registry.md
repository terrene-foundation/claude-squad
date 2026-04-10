# v1.x Issues Registry — Resolved by v2.0 Architecture

## Purpose

Complete registry of known v1.x issues, bugs, and limitations. Each entry maps to how v2.0's architecture resolves it. This ensures nothing falls through the cracks during the rewrite — every issue is either eliminated by architecture, mitigated by design, or explicitly tracked.

## Status Legend

- **ELIMINATED**: v2.0 architecture makes this impossible. No implementation work needed.
- **MITIGATED**: v2.0 architecture makes this natural to fix, but requires correct implementation.
- **SIMPLIFIED**: v2.0 architecture reduces complexity, making the fix straightforward.

---

## Issues Eliminated by Architecture (10)

### ISS-001: Python subprocess tax on statusline render

**Source**: Core architecture
**v1.x behavior**: Each statusline render spawns 3 Python interpreters (~400ms total). With 15 terminals, that's 45 Python starts every few seconds.
**Root cause**: Bash statusline hook calls `python3 rotation-engine.py` as a subprocess for quota update, backsync, and broker check.
**v2.0 resolution**: ELIMINATED. Statusline queries become IPC calls to the daemon (<5ms). No subprocess spawning.
**Verification**: Benchmark statusline latency. Target: <50ms p99.

### ISS-002: No persistent daemon

**Source**: Core architecture
**v1.x behavior**: The broker pattern runs on every statusline render via try-lock. No persistent process. Token refresh, quota polling, and credential sync are all triggered by statusline renders — if no terminal is active, nothing runs.
**Root cause**: v1.x was designed as a stateless CLI tool. The broker pattern was a workaround for not having a daemon.
**v2.0 resolution**: ELIMINATED. Persistent daemon with tokio async runtime. Token refresh, quota polling, and credential sync run on configurable intervals regardless of terminal activity.
**Verification**: Daemon stays alive with 0 terminals open. Token refresh fires on schedule.

### ISS-003: Installation friction

**Source**: install.sh, user reports
**v1.x behavior**: Requires Python 3, jq, curl. Windows needs Git Bash. Users hit PATH issues, Python version conflicts, missing jq on minimal Linux.
**Root cause**: Bash + Python architecture requires runtime dependencies.
**v2.0 resolution**: ELIMINATED. Single compiled binary. Download and run. No runtime dependencies.
**Verification**: Fresh macOS/Linux/Windows install with only the binary. `csq --version` works.

### ISS-004: Windows ctypes handle truncation

**Source**: journal/0013
**v1.x behavior**: Python `ctypes` on Windows truncates 64-bit HANDLE values to 32-bit when `argtypes`/`restype` are not set correctly. This caused `CreateMutexW` to return truncated handles, leading to lock failures.
**Root cause**: Python ctypes defaults to `c_int` (32-bit) for return types. Windows HANDLEs are pointer-sized (64-bit on x64).
**v2.0 resolution**: ELIMINATED. Rust's FFI declarations produce compile-time errors for mismatched types. `HANDLE` is defined as `*mut c_void` (pointer-sized) in the `windows` crate.
**Verification**: Rust compiler rejects mismatched handle types. Existing journal test case should be ported as integration test.

### ISS-005: Non-atomic auto-update

**Source**: csq:76
**v1.x behavior**: `curl -o rotation-engine.py` writes directly to the target file. If curl is interrupted (network drop, Ctrl+C), the file is truncated. Next statusline render spawns Python on a half-written script → crash.
**Root cause**: `rotation-engine.py` is updated with a direct overwrite. `csq` itself uses temp+mv (atomic), but supporting files don't.
**v2.0 resolution**: ELIMINATED. Single binary architecture. Update replaces one file atomically (temp + rename). No supporting scripts to update separately.
**Verification**: Interrupt update mid-download. Binary must remain functional.

### ISS-006: Unsigned auto-updates (supply chain risk)

**Source**: csq:54-84, RT2-C2
**v1.x behavior**: `_auto_update_bg()` downloads from GitHub raw URLs over HTTPS with no signature or checksum verification. MITM at TLS termination point (corporate proxy, DNS hijack) can inject arbitrary code.
**Root cause**: Bash `curl` has no built-in signature verification. Adding GPG/minisign to a bash script adds another dependency.
**v2.0 resolution**: ELIMINATED. Tauri desktop uses Ed25519 signed updates via `tauri-plugin-updater`. CLI-only mode requires checksum verification (DIST-002). Public key embedded in binary.
**Verification**: Tamper with update payload. Binary must reject it.

### ISS-007: Dashboard refresher TOCTOU race

**Source**: dashboard/refresher.py:306-314, RT2-H3
**v1.x behavior**: The dashboard token refresher reads credentials, makes HTTP refresh call, reads credentials again for monotonicity check, then writes — all without holding a file lock. Between post-read and write, another process (rotation-engine broker) can write newer credentials that get overwritten.
**Root cause**: The dashboard refresher was added after the rotation engine and doesn't use the same locking protocol.
**v2.0 resolution**: ELIMINATED. Single daemon centralizes all refresh operations. Per-account `tokio::sync::Mutex` serializes refresh attempts. No file-level TOCTOU possible for the daemon path. CLI fallback mode still acquires the file lock (REL-015).
**Verification**: Concurrent refresh attempts from daemon + CLI fallback. Newer credentials must never be overwritten.

### ISS-008: API key passed via environment variable

**Source**: csq:553, RT2-H4
**v1.x behavior**: `csq setkey` reads the API key from stdin (good), then passes it to an inline Python script via `CSQ_KEY="$key"` environment variable (bad). Env vars are visible via `/proc/{pid}/environ` on Linux and `ps eww` on some systems.
**Root cause**: Bash-to-Python IPC requires either env vars, command-line args, or pipes. The code chose env vars.
**v2.0 resolution**: ELIMINATED. Single Rust binary. Key is read from stdin and processed in the same process. No subprocess, no env var, no IPC.
**Verification**: `ps eww` and `/proc/self/environ` during `csq setkey`. Key must not appear.

### ISS-009: Tokens persist in Python memory allocator

**Source**: dashboard/accounts.py:48, RT2-M5
**v1.x behavior**: OAuth tokens stored as plain Python strings (`self.token = token`). When objects are garbage-collected, the string data remains in Python's memory allocator until the page is reused. No zeroize-on-drop.
**Root cause**: Python has no equivalent to Rust's `secrecy::Secret` with zeroize-on-drop. The GC frees the reference but the bytes persist.
**v2.0 resolution**: ELIMINATED. All token storage uses `secrecy::Secret<String>` with zeroize-on-drop (security analysis S10). When the `Secret` is dropped, the memory is overwritten with zeros before deallocation.
**Verification**: Memory dump after token rotation. Old tokens must not appear in process memory.

### ISS-010: Benchmark env var contamination

**Source**: This session (2026-04-10)
**v1.x behavior**: `test-coc-bench.py` called `os.environ.copy()` which inherited the parent shell's `ANTHROPIC_*` env vars. When running MiniMax benchmarks, the MiniMax model/URL vars leaked into the `claude --print` subprocess, which wrote session state back through symlinked credential files, contaminating other csq accounts.
**Root cause**: Python subprocess inherits parent environment. Bash scripts inherit shell environment. No isolation boundary.
**v2.0 resolution**: ELIMINATED. Rust binary does not inherit shell env vars for model routing. Config is loaded from `settings.json` in the config dir, not from process environment. The benchmark harness strips `ANTHROPIC_*` vars (fixed this session).
**Verification**: Run benchmark with different model. Other terminals must not show the benchmark model.

---

## Issues Mitigated by Design (3)

### ISS-011: Credential cross-contamination

**Source**: journal/0005
**v1.x behavior**: Early versions trusted the `.csq-account` marker file to attribute credential writes. If the marker was stale (from a previous session), credentials were saved to the wrong account slot. Fixed in v1.x by switching to content-based attribution (match by refresh token).
**Root cause**: Marker files are a proxy for identity. The real identity is the credential content itself.
**v2.0 resolution**: MITIGATED. `AccountNum` newtype prevents misattribution at compile time. Credential writes are attributed by content match (refresh token), not marker files. The Rust type system enforces that `AccountNum` values are validated at creation and cannot be confused.
**Implementation needed**: Define `AccountNum` newtype with `TryFrom<u32>`. Use it in all credential function signatures. Preserve the content-match attribution from v1.x's `backsync()`.
**Verification**: Unit test: attempt to write credentials with wrong `AccountNum`. Must fail at compile time or return `Err`.

### ISS-012: Stuck access tokens

**Source**: journal/0011
**v1.x behavior**: An access token passes `claude auth status` (valid JWT, not expired) but fails inference calls (Anthropic-side revocation or session invalidation). The user sees "logged in" but every API call fails with 401. The only fix is `csq login N` to get a fresh token.
**Root cause**: Token validity is not the same as token usability. `auth status` checks local JWT expiry; it doesn't make an inference call.
**v2.0 resolution**: MITIGATED. Daemon can proactively verify tokens with a minimal inference ping (e.g., `claude --print "test" --max-turns 1`). A `csq verify N` command can expose this to the user.
**Implementation needed**: Add `csq verify N` command that makes a minimal API call. Daemon can run periodic verification on a longer interval (e.g., hourly).
**Verification**: Revoke a token server-side. `csq verify N` must detect the failure within one verification cycle.

### ISS-013: WAIT_ABANDONED permanent lock on Windows

**Source**: journal/0013, RT1-H5
**v1.x behavior**: On Windows, `_lock_file()` uses `CreateMutexW`. If the owning process crashes, `WaitForSingleObject` returns `WAIT_ABANDONED` (0x80). v1.x only checks for `WAIT_OBJECT_0` (0x00), treating abandoned as failure. Result: permanently held lock until reboot.
**Root cause**: v1.x Python code doesn't handle the `WAIT_ABANDONED` return value.
**v2.0 resolution**: MITIGATED. Rust can handle `WAIT_ABANDONED` correctly — acquire the lock, log a warning that state may be inconsistent, proceed with the operation.
**Implementation needed**: In `src/platform/lock_windows.rs`, treat `WAIT_ABANDONED` as successful acquisition with `tracing::warn!("lock was abandoned by crashed process")`. Proceed normally.
**Verification**: Start a process that holds the mutex, kill -9 it, then acquire the mutex from another process. Must succeed with warning, not fail.

---

## Issues Simplified by Architecture (3)

### ISS-014: Multiple polling intervals confused

**Source**: dashboard/poller.py, dashboard/refresher.py, rotation-engine.py
**v1.x behavior**: Three separate polling loops with different intervals: Anthropic usage polling (5 min), third-party polling (15 min), token health checks (5 min). These are spread across `poller.py`, `refresher.py`, and the statusline hook in `rotation-engine.py`. The intervals are configured differently and documented inconsistently.
**Root cause**: Polling was added incrementally by different subsystems without centralization.
**v2.0 resolution**: SIMPLIFIED. Single daemon manages all polling intervals. Configuration in one place. Intervals documented in functional requirements (UM-010: Anthropic 5min, 3P 15min, token health 5min).
**Implementation needed**: Define daemon polling config struct with all three intervals. Single `tokio::select!` loop.

### ISS-015: Credential file race on concurrent swap

**Source**: rotation-engine.py
**v1.x behavior**: When two terminals swap simultaneously, both read the credential file, both write their target credentials, and the last writer wins. The rotation engine's per-canonical lock mitigates this for backsync/pullsync, but `csq swap` in bash doesn't acquire the engine's lock — it writes directly.
**Root cause**: `csq swap` bypasses the rotation engine's locking protocol because it's a bash script calling Python inline, not going through the engine's lock acquisition path.
**v2.0 resolution**: SIMPLIFIED. All credential operations go through the daemon (or the in-process core library in CLI fallback mode). The daemon serializes swap requests via `tokio::sync::Mutex`. CLI fallback acquires the file lock. No bypass path.
**Implementation needed**: Ensure `csq swap` in CLI mode acquires the per-account file lock before writing. Daemon mode handles this via the async mutex.

### ISS-016: No desktop app / system tray

**Source**: Vision brief, user request
**v1.x behavior**: Dashboard is a separate `python3 -m dashboard` that the user must start manually and access via browser. No system tray. No native notifications. No at-a-glance quota visibility without a terminal.
**Root cause**: Python + bash architecture has no native desktop integration story.
**v2.0 resolution**: SIMPLIFIED. Tauri provides system tray + webview dashboard out of the box. The daemon IS the desktop app — when running with `--desktop`, it shows the system tray icon. When running headless (`csq daemon start`), it runs without UI.
**Implementation needed**: Phase 3 of the implementation plan. Svelte frontend for dashboard, Tauri system tray menu, native notifications for rate limits and token expiry.

---

## Cross-Reference: Gaps That Need Filling

The following gaps from the red team (04-validate/03-redteam-filtered.md) interact with these issues:

| Gap                            | Related Issues                                         |
| ------------------------------ | ------------------------------------------------------ |
| GAP-1 (Credential JSON schema) | ISS-011 (cross-contamination), ISS-012 (stuck tokens)  |
| GAP-2 (Keychain hex encoding)  | ISS-004 (ctypes handle), ISS-011 (cross-contamination) |
| GAP-4 (Error type hierarchy)   | ISS-013 (WAIT_ABANDONED), ISS-015 (concurrent swap)    |
| GAP-5 (Cargo workspace)        | ISS-001 (subprocess tax), ISS-003 (installation)       |
| GAP-8 (WAIT_ABANDONED)         | ISS-013 (directly)                                     |
| GAP-9 (Daemon detection)       | ISS-002 (no daemon), ISS-014 (polling intervals)       |
