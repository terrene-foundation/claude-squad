# Claude Squad

Account rotation, quota tracking, and profile overlays for Claude Code. Pool multiple Claude Max subscriptions with automatic, quota-aware switching — or use it on a single account just for the statusline and profile overlays. Each terminal isolated, no cross-contamination.

## The problem

Claude Max has rolling rate limits (5-hour and 7-day windows). Heavy users hit these regularly. Manually switching with `/login` interrupts flow and requires guessing which account has capacity.

## What Claude Squad does

- **Auto-rotates** when you hit rate limits — refreshes OAuth token, writes to keychain, CC picks up new creds
- **Per-terminal isolation** — each terminal gets its own keychain entry via `CLAUDE_CONFIG_DIR`, so rotating one terminal doesn't affect others
- **Shared history & memory** — conversations, projects, and auto-memory are symlinked from `~/.claude`, so `/resume` works across all accounts
- **Context & cost in statusline** — see `⚡csq #5:jack 5h:42% | ctx:241k 24% | $5.39` at a glance
- **Smart account picking** — switches to the account with the most available quota
- **Unlimited accounts** — log in as many accounts as you have (1, 7, 20 — no cap)
- **Profile overlays** — start a terminal with a different API provider via `csq run N -p mm` (overlays merge over the canonical default)

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

## Quick start

If you only have **one** Claude account, just run:

```bash
csq          # equivalent to vanilla `claude` — csq stays out of your way
csq --resume # passes flags straight through
```

With zero csq accounts configured, `csq` is invisible — it just execs `claude`.

If you only have one csq account configured, `csq` runs on that account automatically. Once you log in a second account, csq starts asking which one you want.

## Setup (one-time per account)

Save each account's credentials to a numbered slot (any positive integer — 1, 2, 3, … 20, …):

```bash
csq login 1   # opens browser, log in to account 1, saves creds
csq login 2   # repeat for each account
csq login 3
# ...as many as you need
```

You can also save the credentials of an already-logged-in CC session — just run `csq login N` from inside that CC instance and it captures the current keychain entry.

## Daily use

With multiple accounts, start each terminal on a specific one — each gets its own keychain slot:

```bash
csq run 1     # terminal 1 → account 1 (own keychain entry)
csq run 3     # terminal 2 → account 3 (separate keychain entry)
csq run 5     # terminal 3 → account 5 (separate keychain entry)
```

If you have only one account configured, `csq` (no number) auto-resolves it. With zero accounts, `csq` is invisible — it just runs vanilla `claude`.

Any extra arguments are passed straight through to `claude`:

```bash
csq run 5 --resume          # resume the most recent conversation
csq run 5 --resume <id>     # resume a specific session
csq run 3 -p "summarize X"  # one-shot prompt
```

Each terminal survives reboots. The account assignment persists because the keychain entry is tied to the config directory, not the process. Conversation history, projects, and memory are shared across all accounts (symlinked from `~/.claude`), so `/resume` finds the same sessions regardless of which account you're on.

### When rate limited

Inside the rate-limited CC session, type:

```
!csq swap 3       # swap THIS terminal to account 3
```

The `!` prefix runs the command as a local shell op — no LLM call needed, so it works even when CC is rate-limited. The next message you send in CC will automatically use account 3's token, in the same conversation, no restart.

This works because Claude Code picks up updates to `.credentials.json` on its next interaction. `swap_to()` updates the file and the per-config-dir keychain entry, so the swap takes effect right away. Verified empirically.

If you want to know which account to swap to, run `!csq suggest` first.

### From terminal

```bash
csq status              # show all accounts with quota and reset times
csq suggest             # suggest which account to /login to
csq run 4               # start CC on account 4 (default settings)
csq run 4 -p mm         # start CC on account 4 with mm profile overlay
csq run 4 --resume      # resume the most recent conversation
csq swap 3              # in-place swap THIS terminal to account 3
csq cleanup             # remove stale PID cache files
csq help                # full command list
```

## Profile overlays

Profiles are **overlays** at `~/.claude/settings-<name>.json` that get deep-merged onto the canonical `~/.claude/settings.json` at terminal start.

Each profile only needs to contain the diff. Most profiles only need an `env` block to switch API provider. Example `~/.claude/settings-mm.json`:

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "sk-...",
    "ANTHROPIC_MODEL": "MiniMax-M2.7-highspeed"
  }
}
```

When you run `csq run 5 -p mm`, csq:

1. Reads `~/.claude/settings.json` (full default — hooks, statusLine, plugins, etc.)
2. Reads `~/.claude/settings-mm.json` (overlay)
3. Deep-merges them (overlay keys override; nested dicts merge recursively)
4. Writes the result to `config-5/settings.json` as a real file

Result: the mm terminal has the mm API routing AND all the default hooks/statusline/plugins. No duplication, no need to keep multiple full settings files in sync.

**Properties:**

- **No global state.** csq never mutates `~/.claude/settings.json`. The default is the default.
- **No "switch back".** Each `csq run` is fresh. To use mm again, just `csq run N -p mm` again.
- **Stateless per run.** No `.profile` file, no memory between runs.
- **Restart to change profile.** `env` vars are read at process startup, so you can't hot-swap providers.

## How it works

### Per-terminal isolation

Claude Code uses `CLAUDE_CONFIG_DIR` to determine which keychain entry to read/write. The keychain service name is `Claude Code-credentials-<sha256(dir)[:8]>`. Each config directory gets a unique keychain slot.

```
csq run 3
  → CLAUDE_CONFIG_DIR=~/.claude/accounts/config-3
  → keychain: Claude Code-credentials-41cfdf87
  → isolated from all other terminals
```

### Shared artifacts

Only credentials, account identity, and `settings.json` stay isolated. Everything else in `~/.claude` (projects, sessions, history, plugins, commands, agents, skills, memory) is symlinked into each `config-N/` on every `csq run`. So all terminals see the same conversations, the same `/resume` list, and the same auto-memory — only the account and (optionally) the profile change.

Files that stay isolated per config dir:

- `.credentials.json` — OAuth tokens for this terminal's account
- `.current-account` — slot number this terminal is on
- `.claude.json` — onboarding state
- `settings.json` — fresh snapshot built from `~/.claude/settings.json` plus optional `-p` overlay

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
| `csq`                 | `~/.local/bin/`       | CLI: login, run, status, suggest, swap, profile overlays   |
| `statusline-quota.sh` | `~/.claude/accounts/` | Statusline hook: feeds quota to engine, shows account + %  |
| `auto-rotate-hook.sh` | `~/.claude/accounts/` | UserPromptSubmit hook: triggers rotation at 100%           |

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
- One or more Claude accounts (single-account mode is fully supported; rotation needs ≥2)

## Uninstall

```bash
rm -rf ~/.claude/accounts
rm ~/.local/bin/csq          # or ~/bin/csq
# Remove hooks and statusLine from ~/.claude/settings.json
```

## License

Apache 2.0 — [Terrene Foundation](https://terrene.foundation)
