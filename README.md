# Claude Squad

Multi-account rotation for Claude Code. Pool Claude Max subscriptions with automatic, quota-aware switching — each terminal isolated, no cross-contamination.

## The problem

Claude Max has rolling rate limits (5-hour and 7-day windows). Heavy users hit these regularly. Manually switching with `/login` interrupts flow and requires guessing which account has capacity.

## What Claude Squad does

- **Auto-rotates** when you hit rate limits — refreshes OAuth token, writes to keychain, CC picks up new creds
- **Per-terminal isolation** — each terminal gets its own keychain entry via `CLAUDE_CONFIG_DIR`, so rotating one terminal doesn't affect others
- **Quota in statusline** — see `#3:jack 5h:42% 7d:71%` at a glance
- **Smart account picking** — switches to the account with the most available quota
- **Settings profiles** — swap between settings.json variants (e.g., different model configs)

## Install

```bash
curl -sSL https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.sh | bash
```

Or clone and run locally:

```bash
git clone https://github.com/terrene-foundation/claude-squad.git
cd claude-squad
bash install.sh
```

## Setup (one-time per account)

Save each account's credentials to a numbered slot (1–7):

```bash
claude                        # start CC normally
# inside CC: /login email@example.com
# then from the CC prompt: ! csq login 1

# repeat for each account:
# /login another@example.com
# ! csq login 2
```

## Daily use

Start Claude Code on a specific account — each terminal is isolated:

```bash
csq run 1     # terminal 1 → account 1 (own keychain entry)
csq run 3     # terminal 2 → account 3 (separate keychain entry)
csq run 5     # terminal 3 → account 5 (separate keychain entry)
```

Each terminal survives reboots. The account assignment persists because the keychain entry is tied to the config directory, not the process.

### When rate limited

If started via `csq run` (has `CLAUDE_CONFIG_DIR`):

```
/rotate       # auto-swaps: refreshes best account's token, writes to THIS terminal's keychain
```

If started without `csq run`:

```
/rotate       # suggests which account to switch to, you run /login <email>
```

### From terminal

```bash
csq status        # show all accounts with quota and reset times
csq suggest       # suggest which account to /login to
csq run 4         # start CC on account 4
csq use mm        # switch to settings-mm.json profile
csq use default   # switch back to default settings
csq cleanup       # remove stale PID cache files
csq help          # full command list
```

## How it works

### Per-terminal isolation

Claude Code uses `CLAUDE_CONFIG_DIR` to determine which keychain entry to read/write. The keychain service name is `Claude Code-credentials-<sha256(dir)[:8]>`. Each config directory gets a unique keychain slot.

```
csq run 3
  → CLAUDE_CONFIG_DIR=~/.claude/accounts/config-3
  → keychain: Claude Code-credentials-41cfdf87
  → isolated from all other terminals
```

### Auto-rotation flow

```
Statusline fires (each prompt)
  → Feeds rate_limits JSON to rotation engine
  → Engine updates per-account quota in quota.json
  → If 5h usage >= 100% AND CLAUDE_CONFIG_DIR is set:
      → Pick best available account (lowest usage)
      → Refresh that account's OAuth token
      → Write new creds to THIS terminal's keychain entry
      → CC picks up new account on next API call
```

The auto-rotate hook also fires on `UserPromptSubmit` as a backup trigger.

### Token refresh

The engine refreshes OAuth tokens via the public OAuth refresh endpoint:

- Endpoint: `platform.claude.com/v1/oauth/token`
- Grant type: `refresh_token`
- Stored refresh tokens last ~1 year

No browser needed after initial setup.

## Files

| File                  | Installed to          | Purpose                                                    |
| --------------------- | --------------------- | ---------------------------------------------------------- |
| `rotation-engine.py`  | `~/.claude/accounts/` | Core engine: quota tracking, token refresh, keychain write |
| `csq`                 | `~/.local/bin/`       | CLI: login, run, status, suggest, settings swap            |
| `statusline-quota.sh` | `~/.claude/accounts/` | Statusline hook: feeds quota to engine, shows account + %  |
| `auto-rotate-hook.sh` | `~/.claude/accounts/` | UserPromptSubmit hook: triggers rotation at 100%           |
| `rotate.md`           | `~/.claude/commands/` | `/rotate` slash command                                    |

### Data files

| File                                           | Purpose                                                |
| ---------------------------------------------- | ------------------------------------------------------ |
| `~/.claude/accounts/credentials/N.json`        | Stored OAuth creds per account (mode 600)              |
| `~/.claude/accounts/profiles.json`             | Email → account number mapping                         |
| `~/.claude/accounts/quota.json`                | Per-account quota from statusline                      |
| `~/.claude/accounts/config-N/`                 | Per-account CC config dir                              |
| `~/.claude/accounts/config-N/.current-account` | Tracks which account's creds are in this keychain slot |

## Requirements

- macOS (uses macOS Keychain)
- Claude Code CLI
- Python 3
- jq
- Two or more Claude Max subscriptions

## Uninstall

```bash
rm -rf ~/.claude/accounts
rm ~/.local/bin/csq          # or ~/bin/csq
rm ~/.claude/commands/rotate.md
# Remove hooks and statusLine from ~/.claude/settings.json
```

## License

Apache 2.0 — [Terrene Foundation](https://terrene.foundation)
