# csq v2.0 — Full Rust Rewrite

## Vision

Rewrite claude-squad from bash+Python to a single Rust binary with Tauri desktop app. One download, one binary, zero runtime dependencies. Like Ollama but for Claude Code session management.

## What exists today (v1.x)

- `csq` — 1000-line bash script with inline Python for JSON ops
- `rotation-engine.py` — 1900 lines: quota tracking, OAuth token refresh, credential management, keychain integration, atomic file ops, cross-platform support
- `dashboard/` — 2600+ lines Python: HTTP server, account discovery, usage polling, token refresh daemon, OAuth PKCE login
- `statusline-quota.sh` — statusline hook for Claude Code
- `test-coc-bench.py` — 100-point governance benchmark
- `coc-eval/` — 200-point implementation eval harness

## What v2.0 replaces

Everything user-facing. Dev tools (benchmarks, eval harness) stay as Python.

### Current → v2.0 mapping

| Current (bash+Python)    | v2.0 (Rust+Tauri)                                        |
| ------------------------ | -------------------------------------------------------- |
| `csq` bash script        | `csq` Rust binary (CLI mode)                             |
| `rotation-engine.py`     | `src-tauri/src/daemon/` (credentials, refresh, rotation) |
| `dashboard/server.py`    | Tauri backend (built-in HTTP server)                     |
| `dashboard/poller.py`    | Tauri background task                                    |
| `dashboard/refresher.py` | Tauri background task                                    |
| `dashboard/oauth.py`     | Tauri OAuth handler                                      |
| `dashboard/static/`      | Svelte frontend (proper design)                          |
| `statusline-quota.sh`    | CLI query to daemon: `csq quota`                         |

## Key requirements

1. **Single binary** — download and run, no Python/Node required
2. **System tray** — macOS menu bar, Linux/Windows tray. Always running.
3. **Beautiful dashboard** — designed by UI/UX specialist, not hacked together
4. **Central token refresh** — eliminates LOGIN-NEEDED permanently
5. **Pre-emptive rotation** — swap accounts before hitting rate limits
6. **CLI stays powerful** — `csq run`, `csq swap`, `csq models` etc.
7. **Parity with v1.x** — all existing functionality preserved, battle-tested Python as reference
8. **Cross-platform** — macOS, Linux, Windows (Git Bash)
9. **OAuth login via browser** — click "Add Account", authorize, done
10. **5-min polling** — matches claude.ai, keeps sessions alive

## Non-goals

- Not rewriting benchmarks/eval harness (dev tools, stay Python)
- Not building a competing Claude client (csq launches Claude Code, doesn't replace it)
- Not adding inference capabilities (we manage sessions, not make API calls)

## Distribution

- macOS: `.dmg` with app bundle + CLI symlink to `/usr/local/bin/csq`
- Linux: AppImage or `.deb` + CLI in `/usr/local/bin/csq`
- Windows: Installer + CLI in PATH
- All: `curl | sh` installer for CLI-only mode (no desktop app)

## User's constraints

- Terrene Foundation project (Apache 2.0)
- No commercial coupling
- Foundation independence rules apply
