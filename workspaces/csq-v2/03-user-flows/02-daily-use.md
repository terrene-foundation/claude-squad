# User Flow: Daily Use

A typical day working with multiple Claude Code sessions across projects. The user has already installed csq, logged in to multiple accounts, and optionally started the daemon.

---

## Setup Assumed

- csq v2.0 installed (single binary on PATH)
- 5 Anthropic accounts configured (accounts 1-5), each with valid OAuth credentials
- 2 third-party providers configured (MiniMax, ZhipuAI)
- Daemon running (`csq daemon start` or desktop app launched at login)
- Shell prompt configured to show csq statusline (done automatically by `csq install`)

---

## Morning: Start Working

### Open first terminal — Project A

```
~/project-a $ csq run 1
```

Claude Code launches with account 1. The statusline appears in the prompt:

```
~/project-a $ claude                         [1:alice@corp.com 92%]
```

The statusline components:
- `1` — account number
- `alice@corp.com` — email (truncated to fit)
- `92%` — remaining quota for this account's current billing window

Behind the scenes:
1. csq assigned `config-1` to this terminal
2. Wrote `.csq-account = 1` and `.current-account = 1` to `config-1/`
3. Copied fresh credentials from `credentials/1.json` to `config-1/.credentials.json`
4. Notified the daemon: "config-1 is now active on account 1"
5. Launched Claude Code with `CLAUDE_CONFIG_DIR=~/.claude/accounts/config-1`

### Open second terminal — Project B

```
~/project-b $ csq run 2
```

Claude Code launches with account 2 in a separate config directory (`config-2`):

```
~/project-b $ claude                         [2:bob@corp.com 78%]
```

### Open five more terminals

The user opens terminals for projects C through G, assigning accounts 3, 4, 5, 1, 2:

```
~/project-c $ csq run 3                     [3:carol@corp.com 95%]
~/project-d $ csq run 4                     [4:dave@corp.com 88%]
~/project-e $ csq run 5                     [5:eve@corp.com 61%]
~/project-f $ csq run 1                     [1:alice@corp.com 92%]
~/project-g $ csq run 2                     [2:bob@corp.com 78%]
```

Accounts 1 and 2 are shared across multiple terminals. csq handles this:
- Each terminal gets its own `config-N` directory (config-1 through config-7)
- Terminals sharing the same account share the same credentials
- When the daemon refreshes account 1's token, it fans out the new credentials to both config-1 and config-6 (the two directories with `.csq-account = 1`)

---

## Mid-Morning: Check Status

### Quick check from any terminal

```
$ csq status
```

Output (with daemon running — data from in-memory cache, <5ms):

```
Account  Email                 Quota   Reset In   Token    Sessions
───────  ──────────────────    ─────   ────────   ──────   ────────
1        alice@corp.com         92%    4h 12m     valid    2
2        bob@corp.com           78%    3h 45m     valid    2
3        carol@corp.com         95%    4h 30m     valid    1
4        dave@corp.com          88%    4h 02m     valid    1
5        eve@corp.com           61%    2h 18m     valid    1
```

- **Quota**: percentage remaining in the current billing window
- **Reset In**: time until the quota window resets
- **Token**: `valid` (green), `expiring` (yellow, <2 hours), `LOGIN-NEEDED` (red)
- **Sessions**: number of active terminals using this account

### Machine-readable output

```
$ csq status --json
```

Returns structured JSON for scripting and automation.

### Dashboard glance

Click the system tray icon. The dropdown shows a compact version of the same data:

```
csq — 7 active sessions
─────────────────────────
1: alice@corp.com        92%  (2 sessions)
2: bob@corp.com          78%  (2 sessions)
3: carol@corp.com        95%  (1 session)
4: dave@corp.com         88%  (1 session)
5: eve@corp.com          61%  (1 session)
─────────────────────────
Open Dashboard
Stop Daemon
Quit
```

For more detail, click "Open Dashboard" to see the full dashboard with usage graphs, token health indicators, and refresh history.

---

## Midday: Quota Getting Low

### Statusline updates automatically

As the user works, the statusline in each terminal updates on every prompt render. The user notices account 5's quota dropping:

```
~/project-e $ claude                         [5:eve@corp.com 15%]
```

The percentage turns yellow (in terminals that support color) when quota drops below 20%.

### Auto-rotation kicks in

When account 5's quota drops below the auto-rotation threshold (configurable, default 10%), and the user has enabled auto-rotation:

```
~/project-e $ claude                         [3:carol@corp.com 90%]
```

What happened:
1. The daemon detected account 5 at 9% quota
2. `pick_best()` selected account 3 (highest remaining quota among accounts not in heavy use)
3. The daemon swapped config-5 from account 5 to account 3
4. Wrote new credentials to `config-5/.credentials.json`
5. Updated `.csq-account = 3` and `.current-account = 3`
6. The next statusline render shows the new account

The swap is invisible to Claude Code. CC reads `.credentials.json` on its next API call and uses the new token. There is no session restart.

### Manual check before rotation

If auto-rotation is disabled (the default), the user sees the low quota and decides to swap manually:

```
$ csq suggest
```

Output:

```json
{
  "current": 5,
  "suggested": 3,
  "reason": "highest_available_quota",
  "accounts": [
    {"id": 3, "quota_pct": 90, "sessions": 1},
    {"id": 1, "quota_pct": 72, "sessions": 2},
    {"id": 4, "quota_pct": 65, "sessions": 1},
    {"id": 2, "quota_pct": 48, "sessions": 2},
    {"id": 5, "quota_pct": 9, "sessions": 1}
  ]
}
```

Then swap:

```
$ csq swap 3
```

Output:

```
Swapped to account 3 (carol@corp.com). Quota: 90%.
```

The statusline in that terminal updates on the next prompt:

```
~/project-e $ claude                         [3:carol@corp.com 90%]
```

---

## Afternoon: Token Refresh (Invisible)

### Background refresh by the daemon

Anthropic OAuth access tokens expire in approximately 5 hours. The daemon refreshes them proactively — 2 hours before expiry — so the user never sees an expired token.

The user does not see or do anything. The daemon:

1. Checks all accounts every 5 minutes
2. When an account's token is within 2 hours of expiry, acquires the per-account async lock
3. POSTs to Anthropic's token endpoint with the refresh token
4. Receives new access token + new refresh token (Anthropic rotates refresh tokens on each use)
5. Writes the new credentials to the canonical store (`credentials/N.json`)
6. Fans out to every `config-X/.credentials.json` where `.csq-account` matches
7. Writes to the OS keychain (best-effort)
8. Updates the in-memory cache

Claude Code in each terminal picks up the new token on its next API call (it reads `.credentials.json` before each request). No restart needed.

### What the statusline shows during refresh

Nothing changes. The token health indicator stays green because the refresh happens well before expiry. The user's flow is completely uninterrupted.

### If the daemon is not running

If the user chose not to run the daemon, token refresh happens via the broker pattern (same as v1.x): each statusline render triggers a synchronous broker check. The first terminal to render its statusline wins the per-account lock, refreshes the token, and fans out. Other terminals skip (the lock is held) and pick up the refreshed token on their next render cycle.

This is slower (~50ms per statusline render vs ~5ms with daemon) but functionally identical.

---

## Late Afternoon: Check Dashboard for Overview

### Open the dashboard

From the system tray, click "Open Dashboard". The Tauri webview opens.

The dashboard shows:

**Account cards** (one per account):
- Email and account number
- Usage bar: visual representation of quota used vs remaining
- Token health: green dot with "Valid — expires in 3h 12m"
- Active sessions: "2 terminals" with config directory paths
- Last refresh: "14 minutes ago"
- Provider: "Anthropic (OAuth)"

**Third-party accounts** (if configured):
- MiniMax: API key configured, model mm-2.7-highspeed
- ZhipuAI: API key configured, model glm-5.1

**Aggregate view**:
- Total active sessions: 7
- Total quota remaining: 74% (weighted average)
- Estimated time until all accounts exhausted: ~6 hours (based on current usage rate)

### Refresh a specific account

If the user wants to force-refresh an account's token (maybe they suspect it is stale):
- Click the "Refresh" button on that account's card
- The dashboard shows a spinner for 1-2 seconds
- The card updates with the new token expiry time

### Add a new account from the dashboard

If the user wants to add account 6:
1. Click "Add Account" in the dashboard
2. The dashboard starts an OAuth PKCE flow:
   - Generates code verifier and challenge
   - Opens the browser to Anthropic's authorize URL
   - The user signs in with a new Anthropic account
   - Anthropic redirects back to `http://127.0.0.1:8420/oauth/callback`
   - The daemon exchanges the code for tokens
   - Credentials saved to `credentials/6.json`
3. The new account appears in the dashboard immediately
4. The daemon begins polling its usage and managing its token refresh

---

## Using Third-Party Models

### Switch a session to MiniMax

```
$ csq models switch mm-2.7-highspeed
```

This updates the active config directory's settings to use MiniMax instead of Anthropic:
- Sets `ANTHROPIC_AUTH_TOKEN` to the MiniMax API key
- Sets `ANTHROPIC_BASE_URL` to MiniMax's endpoint
- Sets `ANTHROPIC_MODEL` to `mm-2.7-highspeed`

The statusline shows the provider:

```
~/project-a $ claude                         [MM:mm-2.7 ∞]
```

- `MM` — provider abbreviation
- `mm-2.7` — model short name
- `∞` — third-party providers do not have the same quota system (rate limits shown if available)

### Switch back to Anthropic

```
$ csq models switch opus
```

Restores Anthropic OAuth credentials and model settings.

---

## End of Day: Wrap Up

### Close terminals

The user closes their terminals normally (`exit` or Ctrl-D). Claude Code exits. csq does not need any cleanup — the config directories persist for next time.

### Daemon keeps running

The daemon continues running in the background, keeping tokens fresh. When the user opens terminals tomorrow, credentials are already valid and current.

### Stop the daemon (optional)

```
$ csq daemon stop
```

Or quit from the system tray menu. The daemon shuts down gracefully. All state is persisted to disk (credentials, quota, profiles). Starting the daemon again tomorrow is instant — it reads the persisted state and resumes.

### Desktop app auto-start

If the user installed the desktop app and enabled auto-start (from Settings), the daemon and system tray launch automatically at login. The user does not need to run `csq daemon start` manually.

---

## Statusline Reference

The statusline format adapts to conditions:

| Condition | Statusline | Meaning |
| --- | --- | --- |
| Normal | `[1:alice@corp.com 85%]` | Account 1, 85% quota remaining |
| Low quota | `[1:alice@corp.com 12%]` | Yellow — approaching limit |
| Very low | `[1:alice@corp.com 3%]` | Red — swap recommended |
| After swap | `[3:carol@corp.com 90%]` | Swapped to account 3 |
| Token expiring | `[1:alice@corp.com 85% !T]` | Token expires within 30 minutes |
| Login needed | `[LOGIN-NEEDED:1]` | Token refresh failed, re-login required |
| Broker failure | `[!B 1:alice@corp.com 85%]` | Broker marked failed (still usable, refresh broken) |
| Third-party | `[MM:mm-2.7 ∞]` | MiniMax provider, no quota tracking |
| Context window | `[1:alice 85% 42k/200k]` | With token count (42k used of 200k context) |
| No daemon | Same as above | All formats work identically without daemon |

---

## Key Differences from v1.x Daily Use

| Aspect | v1.x | v2.0 |
| --- | --- | --- |
| Statusline speed | ~400ms (3 Python subprocesses) | ~5ms (daemon IPC) or ~50ms (direct) |
| Token refresh | Broker subprocess on each render | Background daemon, proactive, invisible |
| Dashboard access | `python -m dashboard`, then open browser | System tray click, or `127.0.0.1:8420` |
| Adding accounts | CLI only (`csq login N`) | CLI or dashboard (OAuth flow in browser) |
| Swap speed | ~100ms (Python engine) | ~20ms (direct) or ~5ms (daemon) |
| Memory per terminal | ~30MB (Python interpreter) | ~0MB (csq exits after exec, Claude Code is the process) |
| Concurrent terminals | 15 tested | 50+ (per-account async locks) |
| Auto-rotation | Not available | Optional, configurable threshold |
