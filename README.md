# Claude Squad

Intelligent multi-account rotation for Claude Code. Maximize your throughput by pooling Claude Max subscriptions with automatic, quota-aware switching.

## The problem

Claude Max subscriptions have rolling rate limits (5-hour and 7-day windows). Heavy users hit these limits regularly, stalling work mid-session. Manually switching accounts with `/login` interrupts flow and requires guessing which account has capacity.

## What Claude Squad does

- **Auto-rotates** when rate limits hit — no manual intervention, no browser
- **Drains accounts smartly** — prioritizes accounts whose weekly quota expires soonest (use-it-or-lose-it)
- **Coordinates across terminals** — multiple concurrent sessions share the account pool without conflicts
- **Shows quota in the statusline** — see usage percentages and reset timers at a glance
- **One-time setup** — credentials persist for ~1 year after initial login

## Install

```bash
curl -sfL https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.sh | bash
```

Or clone and run:

```bash
git clone https://github.com/terrene-foundation/claude-squad.git
cd claude-squad
bash install.sh
```

The installer walks you through logging in to each account (one-time browser auth per account).

### Add accounts later

```bash
ccc login 3     # Add account to slot 3 (browser login, one-time)
```

## How it works

### Priority algorithm

**Use-it-or-lose-it**: accounts with weekly quota resetting soonest get drained first.

```
Account A: weekly resets in 1 day  → HIGH PRIORITY (use before it resets)
Account B: weekly resets in 5 days → low priority (save for later)
```

When an account hits its 5-hour rate limit, it's temporarily parked. The system switches to the next best account. When the 5-hour window resets, it switches back if that account still has the highest priority.

### Multi-terminal coordination

Each Claude Code session claims an account via a lockfile-coordinated assignment table. The system load-balances — sessions spread across accounts, preferring those with fewer active users.

### Auto-rotation flow

```
Statusline renders (every few seconds)
  → Captures rate_limits from Claude Code
  → Updates shared quota state
  → Checks: should this session rotate?
  → If yes: swaps Keychain credentials silently
  → Next API call uses the new account
```

No user action required. No browser. No `/login`.

### Polling all accounts

```bash
ccc refresh     # Poll all accounts in parallel to check availability
```

Each account is queried using its stored refresh token in a sandboxed `claude -p` call. Results update the shared quota state.

## Usage

### Inside Claude Code

Fully automatic. When you hit a rate limit, it rotates for you.

Manual rotation is also available:

```
/rotate         # Show recommendation + rotate if needed
```

### From terminal

```bash
ccc quota       # Show all accounts with quota, priority, terminal assignments
ccc refresh     # Poll all accounts for current quota
ccc swap 3      # Manually switch to account 3
ccc status      # List configured accounts
ccc login 4     # Add a new account (browser, one-time)
ccc help        # Full command list
```

## Files installed

| File | Location | Purpose |
|---|---|---|
| `rotation-engine.py` | `~/.claude/accounts/` | Core engine: quota tracking, priority, credential swap |
| `ccc` | `~/bin/` or `~/.local/bin/` | CLI for account management |
| `auto-rotate-hook.sh` | `~/.claude/accounts/` | Hook: checks rotation on each user message |
| `statusline-quota.sh` | `~/.claude/accounts/` | Statusline: displays quota + feeds data to engine |
| `rotate.md` | `~/.claude/commands/` | `/rotate` slash command for manual trigger |

### Runtime data

| File | Purpose |
|---|---|
| `~/.claude/accounts/credentials/N.json` | OAuth credentials per account (mode 600) |
| `~/.claude/accounts/quota-state.json` | Real-time quota data from all sessions |
| `~/.claude/accounts/assignments.json` | Session-to-account mapping |
| `~/.claude/accounts/profiles.json` | Account emails and metadata |
| `~/.claude/accounts/rotation-history.jsonl` | Audit log of all rotations |

## Requirements

- macOS (uses macOS Keychain for credential storage)
- Claude Code CLI
- Python 3
- jq
- Two or more Claude Max subscriptions

## How credentials work

Each account's OAuth credentials (access token + refresh token) are extracted from the macOS Keychain after a one-time browser login. The refresh token is long-lived (~1 year) — Claude Code silently renews the access token as needed.

On rotation, the engine writes the target account's credentials to the Keychain and touches `~/.claude/.credentials.json` to trigger Claude Code's credential cache invalidation. The next API call seamlessly uses the new account.

## Uninstall

```bash
rm -rf ~/.claude/accounts
rm ~/bin/ccc  # or ~/.local/bin/ccc
rm ~/.claude/commands/rotate.md
# Remove the UserPromptSubmit hook from ~/.claude/settings.json manually
```

## License

Apache 2.0 — [Terrene Foundation](https://terrene.foundation)
