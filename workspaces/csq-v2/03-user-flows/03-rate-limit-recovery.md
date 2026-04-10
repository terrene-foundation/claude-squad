# User Flow: Rate Limit Recovery

What happens when a Claude Code session hits a rate limit, and how csq recovers — automatically and manually. Covers the full spectrum from a single account hitting its limit to all accounts exhausted.

---

## Setup Assumed

- csq v2.0 installed, daemon running
- 5 Anthropic accounts configured (accounts 1-5)
- 7 active Claude Code sessions across those accounts
- Account 2 is on its last 5% of quota, being used in 2 terminals (config-3 and config-7)

---

## Scenario 1: Single Account Hits Rate Limit

### What triggers it

The user is working in project-b (config-3, account 2). Claude Code sends a request to Anthropic's API and receives a `429 Too Many Requests` response. The response includes a `retry-after` header indicating when the quota window resets.

### What happens automatically

#### Step 1: Claude Code reports the rate limit

Claude Code's own rate limit handling displays a message in the terminal:

```
Rate limit reached. Retrying in 45 minutes...
```

Claude Code will retry automatically, but the user will be waiting for 45 minutes.

#### Step 2: csq detects the rate limit (next statusline render)

On the next prompt render (within seconds), `csq statusline` queries the daemon for account 2's status. The daemon has already picked up the 429 from its usage polling cycle and marked account 2 as rate-limited.

The statusline changes:

```
~/project-b $ claude                         [2:bob@corp.com 0% 45m]
```

- `0%` — quota exhausted
- `45m` — time until quota window resets

#### Step 3: Auto-rotation (if enabled)

If the user has enabled auto-rotation (`csq config set auto-rotate true`), the daemon acts immediately:

1. Detects account 2 is at 0% quota
2. Calls `pick_best()` to find the account with the highest remaining quota that is not also exhausted
3. Selects account 4 (88% remaining, 1 session)
4. Swaps config-3 from account 2 to account 4:
   - Writes `credentials/4.json` contents to `config-3/.credentials.json` (atomic)
   - Updates `.csq-account = 4`
   - Updates `.current-account = 4`
   - Writes to keychain (best-effort)
5. Also swaps config-7 (the other terminal on account 2) to account 4

The statusline updates on the next prompt:

```
~/project-b $ claude                         [4:dave@corp.com 88%]
```

Claude Code's next API call uses account 4's credentials. The rate limit is bypassed because it is a different account. The user continues working without interruption.

#### What if auto-rotation is disabled?

The statusline shows the rate limit, but csq does not swap automatically. The user sees:

```
~/project-b $ claude                         [2:bob@corp.com 0% 45m]
```

The user decides what to do (see Manual Recovery below).

### What the user sees

With auto-rotation enabled: the statusline briefly flashes to `0%`, then changes to a new account with high quota. The transition takes less than 1 second. Claude Code does not restart — it simply uses the new credentials on its next request.

Without auto-rotation: the user sees the `0%` indicator and the reset timer. They can swap manually or wait.

---

## Scenario 2: Manual Recovery — `csq swap N`

### The user chooses to swap

The user sees account 2 is rate-limited and decides to switch to account 3:

```
$ csq swap 3
```

What happens:
1. csq reads `credentials/3.json` (account 3's cached credentials)
2. Writes them to the current config directory's `.credentials.json` (atomic)
3. Updates `.csq-account = 3` and `.current-account = 3`
4. Writes to keychain (best-effort)
5. Starts a background verification thread that checks at +2 seconds whether Claude Code overwrote the swap (this handles a race where CC might be mid-refresh)
6. Prints:
   ```
   Swapped to account 3 (carol@corp.com). Quota: 90%.
   ```

The statusline updates:

```
~/project-b $ claude                         [3:carol@corp.com 90%]
```

Claude Code's next API call succeeds with account 3's credentials.

### Choosing the best account to swap to

If the user does not know which account has the most quota:

```
$ csq suggest
```

Output:

```json
{
  "current": 2,
  "suggested": 4,
  "reason": "highest_available_quota",
  "accounts": [
    {"id": 3, "quota_pct": 90, "sessions": 1},
    {"id": 4, "quota_pct": 88, "sessions": 1},
    {"id": 1, "quota_pct": 72, "sessions": 2},
    {"id": 5, "quota_pct": 45, "sessions": 1},
    {"id": 2, "quota_pct": 0, "sessions": 2, "reset_in": "45m"}
  ]
}
```

The user sees account 3 has the highest quota and swaps to it.

### Swap from the system tray

The user can also swap from the system tray menu without using the terminal:

1. Click the tray icon
2. The menu shows all accounts with their quota
3. Right-click on the affected terminal's entry
4. Select "Swap to Account 3"

The swap happens immediately. The statusline updates in the terminal.

---

## Scenario 3: Token Refresh After Rate Limit

### The rate-limited account's token is still valid

Rate limiting is about usage quota, not token validity. Account 2's OAuth token is still valid — it just cannot make API calls until the quota window resets. The daemon continues refreshing account 2's token as normal. When the quota resets (in 45 minutes), account 2 becomes available again.

### The user swaps back after reset

After the quota window resets:

```
$ csq status
```

```
Account  Email              Quota   Reset In   Token    Sessions
───────  ────────────────   ─────   ────────   ──────   ────────
1        alice@corp.com      65%    3h 45m     valid    2
2        bob@corp.com        95%    4h 55m     valid    0     ← quota reset, fresh window
3        carol@corp.com      82%    4h 12m     valid    2
4        dave@corp.com       80%    3h 58m     valid    2
5        eve@corp.com        38%    1h 30m     valid    1
```

Account 2 has reset to 95% (a new quota window started). The user can swap back:

```
$ csq swap 2
```

---

## Scenario 4: Token Refresh Failure (LOGIN-NEEDED)

### What triggers it

A different kind of failure: the daemon tries to refresh account 2's OAuth token and the refresh fails. This happens when:

- Anthropic rotated the refresh token and Claude Code's internal refresh won the race (the daemon's stored refresh token is now invalid)
- The user revoked the OAuth authorization from Anthropic's account settings
- Anthropic's OAuth endpoint is temporarily down

### What happens automatically

#### Step 1: Daemon detects refresh failure

The daemon's refresh attempt returns a 401 (invalid refresh token). The daemon:

1. Does NOT panic. Enters the recovery path.
2. Scans all `config-X/.credentials.json` files where `.csq-account = 2`
3. Looks for a refresh token that differs from the canonical one (this means CC refreshed it independently)
4. If found: promotes the live sibling's credentials to canonical, retries the refresh
5. If the retry succeeds: recovery complete, credentials fanned out, no user action needed

#### Step 2: Recovery succeeds (common case)

The daemon found a live config directory where Claude Code had already refreshed the token. It promoted that token, refreshed successfully, and fanned out. The user sees nothing — the statusline stays green.

#### Step 3: Recovery fails (rare case)

All refresh tokens are dead. The daemon:

1. Marks the account as LOGIN-NEEDED by creating `credentials/2.broker-failed`
2. Sends a system tray notification: "Account 2 (bob@corp.com) needs re-login"
3. Updates the statusline for all terminals on account 2

The statusline changes:

```
~/project-b $ claude                         [LOGIN-NEEDED:2]
```

The dashboard shows account 2's token health as red with "Re-login required."

### Manual recovery: re-login

The user re-authenticates:

```
$ csq login 2
```

This opens the browser for a fresh OAuth flow. After signing in:

1. New credentials saved to `credentials/2.json`
2. Fanned out to all config directories on account 2
3. The `broker-failed` flag is cleared
4. The daemon resumes normal refresh cycles
5. The statusline returns to normal:
   ```
   ~/project-b $ claude                       [2:bob@corp.com 95%]
   ```

### Recovery from the dashboard

Instead of the CLI, the user can re-login from the dashboard:

1. Open dashboard (tray -> "Open Dashboard")
2. Account 2's card shows a red "Re-login Required" badge
3. Click "Re-login" on account 2's card
4. Browser opens to Anthropic's authorization page
5. User signs in
6. Callback handled by the daemon at `http://127.0.0.1:8420/oauth/callback`
7. Credentials saved, account restored

---

## Scenario 5: All Accounts Exhausted

### What triggers it

The user has been working intensively across all 5 accounts. All accounts are now at 0% quota or rate-limited. There is nowhere to rotate to.

### What the user sees

```
$ csq status
```

```
Account  Email              Quota   Reset In   Token    Sessions
───────  ────────────────   ─────   ────────   ──────   ────────
1        alice@corp.com       0%    32m        valid    2
2        bob@corp.com         0%    45m        valid    2
3        carol@corp.com       0%    18m        valid    1
4        dave@corp.com        2%    55m        valid    1
5        eve@corp.com         0%    1h 05m     valid    1
```

The statusline in all terminals shows 0%:

```
~/project-a $ claude                         [1:alice@corp.com 0% 32m]
```

### What csq does

1. **Auto-rotation does not swap**: `pick_best()` finds no account with quota above the threshold. It returns the current account (no change).
2. **Suggest shows the situation**: `csq suggest` returns all accounts at 0% with their reset times.
3. **The daemon waits**: No rotation possible. The daemon continues monitoring and will resume rotation when the first account's quota window resets.

### What the user can do

**Option A: Wait for the earliest reset.**

Account 3 resets in 18 minutes. After it resets:
- The daemon detects account 3 is now available
- If auto-rotation is enabled, swaps the most affected terminal to account 3
- If not, `csq suggest` will recommend account 3

**Option B: Use a third-party provider.**

```
$ csq models switch mm-2.7-highspeed
```

This switches the terminal to use MiniMax (or another configured provider). Third-party providers have their own rate limits and are independent of Anthropic's quota. The statusline shows:

```
~/project-a $ claude                         [MM:mm-2.7 ∞]
```

The user can work on MiniMax while waiting for Anthropic accounts to reset, then switch back:

```
$ csq models switch opus
```

**Option C: Add another account.**

```
$ csq login 6
```

Or from the dashboard, click "Add Account." A new Anthropic account with fresh quota is available immediately.

---

## Scenario 6: Rate Limit During Daemon-Offline Operation

### What triggers it

The daemon is not running (the user chose CLI-only mode, or the daemon crashed). The user hits a rate limit in one of their terminals.

### How it differs

Without the daemon:
- No automatic detection of rate limits from usage polling
- No automatic rotation
- The user discovers the rate limit when Claude Code reports it
- Statusline still works (computes quota from disk, ~50ms), but quota data may be slightly stale

### Manual recovery (identical)

The user runs the same commands:

```
$ csq suggest          # see which account has quota
$ csq swap 3           # swap to that account
```

The swap works identically — it reads and writes credential files directly, without going through the daemon. The only difference is speed (~20ms vs ~5ms) and the absence of automatic rotation.

### Starting the daemon after the fact

If the user wants automatic recovery going forward:

```
$ csq daemon start
```

The daemon starts, reads the current state from disk, and begins managing all accounts. Any currently rate-limited accounts are detected and handled in the next polling cycle.

---

## Recovery Timelines

| Scenario | Recovery Time | User Action Required |
| --- | --- | --- |
| Single account rate-limited, auto-rotation on | < 1 second | None |
| Single account rate-limited, auto-rotation off | Seconds (manual swap) | `csq swap N` |
| Token refresh failed, live sibling has good token | < 5 seconds (daemon recovery) | None |
| Token refresh failed, all tokens dead | Minutes (re-login) | `csq login N` |
| All accounts exhausted | 18-60 minutes (wait for reset) | Wait, or switch provider, or add account |
| All accounts exhausted + no third-party provider | Until first account resets | Wait |

---

## System Tray Notifications

The system tray icon provides visual feedback for rate limit events:

| Event | Tray Notification | Tray Icon Change |
| --- | --- | --- |
| Account rate-limited | "Account 2 rate-limited. Auto-rotated to account 4." | Badge shows count of rate-limited accounts |
| Token refresh failed + recovered | None (invisible) | No change |
| Token refresh failed + LOGIN-NEEDED | "Account 2 needs re-login." | Red badge |
| All accounts exhausted | "All accounts rate-limited. Next reset in 18m." | Warning badge |
| Account quota reset | "Account 3 quota reset (95%)." | Badge cleared (if no other issues) |

---

## Design Decisions Behind Rate Limit Recovery

### Why the daemon refreshes tokens proactively (2-hour window)

Anthropic access tokens expire in ~5 hours. The daemon refreshes them when there are 2 hours remaining. This wide window serves two purposes:

1. **Prevents Claude Code from ever seeing an expired token.** If CC never sees an expired token, it never triggers its own internal refresh path. This eliminates the race condition where CC and csq both try to refresh the same token simultaneously (and Anthropic's single-use refresh token causes one of them to fail).

2. **Gives time for recovery.** If the first refresh attempt fails (network issue, temporary Anthropic outage), the daemon has 2 hours to retry before the token actually expires. With a 5-minute check interval, that is 24 retry opportunities.

### Why swap does not trigger a token refresh

When `csq swap N` writes account N's credentials to a config directory, it writes the cached credentials from `credentials/N.json`. It does NOT call the refresh endpoint. This is intentional:

1. **Speed**: swap completes in <20ms. A refresh would add 500ms-2s.
2. **No race**: swap is a file write. It does not interact with Anthropic's API. No chance of token rotation race.
3. **Daemon handles freshness**: if the cached credentials are stale, the daemon will refresh them on its next check cycle (within 5 minutes). The user does not wait.

### Why all accounts exhausted is not treated as an error

When all accounts are at 0%, csq does not raise an alarm or force the user to do anything. It shows the state clearly and lets the user decide. The user may:
- Wait (accounts reset on their own)
- Switch providers (if configured)
- Add more accounts
- Take a break

Forcing action would be worse than showing the situation. The user knows their workflow best.
