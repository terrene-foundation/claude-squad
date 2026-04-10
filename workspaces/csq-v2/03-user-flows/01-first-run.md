# User Flow: First Run

The new user experience from download to first Claude Code session. Everything happens through a single binary — no Python, no jq, no bash dependencies.

---

## Prerequisites

- A machine running macOS (arm64 or x86_64), Linux (x86_64 or arm64), or Windows (x86_64)
- Claude Code CLI installed (`claude` command available)
- At least one Anthropic account with active subscription

---

## Step 1: Download and Install

The user downloads the csq binary. One command, one file.

### macOS / Linux

```
curl -sSL https://csq.terrene.dev/install | sh
```

What happens:
1. The script detects the platform (macOS-arm64, macOS-x86_64, Linux-x86_64, Linux-arm64)
2. Downloads the correct `csq` binary from GitHub Releases (signed, checksum-verified)
3. Places it in `~/.local/bin/csq` (creates the directory if needed)
4. Makes it executable (`chmod +x`)
5. Runs `csq install` to configure Claude Code integration (see Step 2)
6. Prints: "csq installed. Run `csq login 1` to add your first account."

If `~/.local/bin` is not on PATH, the installer prints instructions to add it.

### Windows

```
scoop install csq
```

Or download `csq.exe` from the GitHub Releases page and place it on PATH.

### macOS (Homebrew)

```
brew install terrene-foundation/tap/csq
```

### Desktop App (All Platforms)

Download the desktop installer from the Releases page:
- macOS: `csq-2.0.0-arm64.dmg` or `csq-2.0.0-x86_64.dmg`
- Windows: `csq-2.0.0-x64-setup.exe` (NSIS installer)
- Linux: `csq-2.0.0-x86_64.AppImage` or `csq-2.0.0-amd64.deb`

The desktop installer includes the CLI binary. Installing the desktop app also gives you the `csq` command.

---

## Step 2: `csq install` — Automatic Configuration

`csq install` runs automatically after download (or the user can run it manually). It configures Claude Code to use csq for quota display.

What happens:
1. Creates `~/.claude/accounts/` and `~/.claude/accounts/credentials/` with `0700` permissions if they do not exist
2. Patches Claude Code's `settings.json` to set the statusline command:
   ```json
   {
     "env": {
       "CLAUDE_STATUSLINE_COMMAND": "csq statusline"
     }
   }
   ```
3. Verifies that `claude` is on PATH. If not, prints a warning (csq still works, but `csq run` cannot launch Claude Code)
4. Prints a summary:
   ```
   csq v2.0.0 installed
   Claude Code: /usr/local/bin/claude (v1.x.x)
   Statusline: configured
   Accounts: 0 (run `csq login 1` to add your first account)
   ```

If v1.x csq artifacts are detected (the old `statusline-quota.sh`, Python rotation engine), `csq install` migrates automatically:
- Replaces the old statusline command
- Removes dead scripts (statusline-quota.sh, rotate.md, auto-rotate-hook.sh)
- Preserves all credential files and profiles (same format, no conversion needed)

---

## Step 3: Add First Account — `csq login 1`

The user adds their first Anthropic account. The `1` is the account number (users with multiple accounts will repeat this with `2`, `3`, etc.).

```
csq login 1
```

What happens:
1. csq creates an isolated config directory: `~/.claude/accounts/config-1/`
2. csq runs `claude auth login` with `CLAUDE_CONFIG_DIR` set to that directory
3. Claude Code's OAuth flow starts:
   - The terminal prints "Opening browser for authentication..."
   - The default browser opens to Anthropic's login page
   - The user signs in with their Anthropic credentials
   - Anthropic redirects back to a local callback URL
   - Claude Code exchanges the authorization code for OAuth tokens (using PKCE)
4. csq captures the credentials:
   - First, it reads from the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager) using the SHA256-derived service name that Claude Code uses
   - If keychain read fails, it reads from `config-1/.credentials.json` directly
5. csq saves the credentials:
   - Canonical store: `~/.claude/accounts/credentials/1.json` (atomic write, `0600` permissions)
   - Live store: `~/.claude/accounts/config-1/.credentials.json` (atomic write, `0600` permissions)
   - Keychain: writes to the OS keychain as a backup (best-effort, never blocks)
6. csq captures the account email:
   - Runs `claude auth status --json` to extract the authenticated email
   - Saves to `~/.claude/accounts/profiles.json`: `{ "accounts": { "1": { "email": "user@example.com", "method": "claude_auth" } } }`
7. Prints confirmation:
   ```
   Account 1 logged in: user@example.com
   Credentials saved to ~/.claude/accounts/credentials/1.json
   
   Run `csq run` to start a Claude Code session with this account.
   ```

### What if the user has a second account?

```
csq login 2
```

Same flow, different account number. The second login opens a fresh browser context so the user can sign in with a different Anthropic account. Each account gets its own credential file and config directory.

### What if the user has a third-party API key (not Anthropic OAuth)?

```
csq setkey mm
```

This starts an interactive flow to configure a MiniMax (or other provider) API key:
1. Prompts for API key
2. Validates the key with a test API call
3. Saves provider config to `~/.claude/accounts/settings-mm.json`
4. Sets default model for the provider

---

## Step 4: First Claude Code Session — `csq run`

The user runs their first Claude Code session through csq.

```
csq run
```

Or, equivalently, with an explicit account number:

```
csq run 1
```

What happens:
1. **Account resolution**:
   - If the user has exactly 1 account, csq uses it automatically
   - If the user specified `csq run 1`, csq uses account 1
   - If the user has 2+ accounts and did not specify, csq prints an error: "Multiple accounts configured. Specify an account number: `csq run 1`"
   - If the user has 0 accounts, csq runs vanilla `claude` (no account management)

2. **Config directory setup**:
   - Assigns a config directory: `~/.claude/accounts/config-1/` (or the next available one if 1 is in use by another terminal)
   - Writes the `.csq-account` marker (account identity for this directory)
   - Writes the `.current-account` marker (for statusline display)
   - Copies credentials from `credentials/1.json` to `config-1/.credentials.json` (atomic write)

3. **Session isolation**:
   - Symlinks shared Claude Code artifacts (projects, todoItems, CLAUDE.md cache) from the default `~/.claude/` into the config directory so the user keeps their project context
   - Creates an isolated `.claude.json` in the config directory with the onboarding flag set (so Claude Code does not show the first-run wizard)
   - Deep-merges any settings overlay (model, provider) into the config directory's settings

4. **Broker check**:
   - If a daemon is running, notifies it that a new session is starting on account 1
   - If no daemon is running, runs a synchronous broker check: verifies credentials are fresh, refreshes if expiring within 2 hours, fans out to all config directories using this account

5. **Environment preparation**:
   - Sets `CLAUDE_CONFIG_DIR` to the assigned config directory
   - Strips `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` from the environment (prevents conflicts with OAuth credentials)
   - Preserves all other environment variables

6. **Launch Claude Code**:
   - Replaces the csq process with `claude` (Unix `exec`, Windows `spawn + wait`)
   - Claude Code starts with the isolated config directory, authenticated credentials, and full project context

### What the user sees

The terminal shows Claude Code starting normally. The only visible difference from running `claude` directly is the statusline in the prompt (if the user has configured their shell's RPROMPT):

```
~/project $ claude                           [1:user@example.com 85%]
```

The statusline shows:
- `1` — the account number
- `user@example.com` — truncated email
- `85%` — remaining quota percentage

---

## Step 5: See the Dashboard (Optional)

After running at least one session, the user can see the dashboard in two ways.

### Option A: System Tray (Desktop App)

If the user installed the desktop app:
1. The system tray icon appears automatically when the daemon starts
2. Clicking the tray icon shows a menu:
   ```
   csq — 1 active session
   ─────────────────────────
   Account 1: user@example.com     85%  [Active]
   Account 2: other@example.com    92%
   ─────────────────────────
   Open Dashboard
   Start Daemon
   Quit
   ```
3. Clicking "Open Dashboard" opens the Tauri webview window

### Option B: Browser Dashboard

If the user prefers the CLI:

```
csq daemon start
```

Then open `http://127.0.0.1:8420` in any browser.

### What the dashboard shows

The dashboard is a single-page app with live-updating data:

**Accounts panel**:
- Each account shown as a card with: email, account number, provider, remaining quota (progress bar), token health indicator (green/yellow/red), last refresh time
- Active sessions highlighted (which terminals are using which account)

**Token health panel**:
- Green: token valid, >2 hours until expiry
- Yellow: token valid, <2 hours until expiry (refresh imminent)
- Red: token expired or refresh failed (LOGIN-NEEDED)
- Each account shows its token expiry countdown

**Quick actions**:
- "Refresh Now" button per account (forces immediate token refresh)
- "Add Account" button (starts OAuth login flow in the browser)
- "Swap" dropdown to change which account a specific config directory uses

---

## Step 6: Run the Daemon for Background Management (Optional)

The daemon provides background token refresh and faster statusline rendering. It is optional — everything works without it.

### Start the daemon

```
csq daemon start
```

What happens:
1. The daemon starts as a background process
2. Creates a PID file at `~/.claude/accounts/csq.pid`
3. Opens a Unix socket at `/tmp/csq-{uid}.sock` (macOS/Linux) or named pipe `\\.\pipe\csq-{username}` (Windows)
4. Begins background work:
   - Token refresh: checks every 5 minutes, refreshes tokens 2 hours before expiry
   - Usage polling: polls Anthropic API for quota data on each account
   - Serves the HTTP API on `127.0.0.1:8420`
5. Prints: "Daemon started (PID 12345). Dashboard at http://127.0.0.1:8420"

Once the daemon is running, all CLI commands become faster:
- `csq status` reads from the daemon's in-memory cache instead of disk (~5ms vs ~30ms)
- `csq statusline` gets data from the daemon via IPC instead of computing it (~5ms vs ~50ms)
- Token refresh happens proactively — terminals never see expired tokens

### Stop the daemon

```
csq daemon stop
```

Graceful shutdown: finishes in-flight refresh operations, closes the socket, removes the PID file. All CLI commands continue working (they fall back to direct file access).

---

## Summary: What Changed from v1.x

| Aspect | v1.x | v2.0 |
| --- | --- | --- |
| Install | `curl | bash`, requires Python 3, jq | `curl | sh`, single binary, zero dependencies |
| Login | Same `csq login N` command | Same command, faster (no Python startup) |
| Run | Same `csq run N` command | Same command, <200ms to launch (vs ~1.5s) |
| Statusline | Bash script spawning 3 Python processes (~400ms) | Single binary IPC call (~5ms with daemon, ~50ms without) |
| Dashboard | `python -m dashboard` (manual start, browser-only) | System tray + `csq daemon start` (auto-start available) |
| Token refresh | Broker subprocess on every statusline render | Background daemon, proactive 2-hour window |
| Config files | Identical format | Identical format (full backwards compatibility) |
