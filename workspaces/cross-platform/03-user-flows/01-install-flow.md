# User Flow: Installation

## macOS User

```
$ curl -sSL https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.sh | bash

Detected: macOS
✓ claude found
✓ python3 found
✓ jq found
✓ Files installed to ~/.claude/accounts/
✓ csq installed to ~/.local/bin/
✓ Keychain entries initialized for 7 slots
✓ Settings configured (statusline + auto-rotate hook)

Run: csq login 1
```

## Linux User

```
$ curl -sSL https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.sh | bash

Detected: Linux
✓ claude found
✓ python3 found
✓ jq found (if missing: sudo apt install jq)
✓ Files installed to ~/.claude/accounts/
✓ csq installed to ~/.local/bin/
  Skipping keychain setup (file-only credential storage on Linux)
✓ Settings configured (statusline + auto-rotate hook)

Run: csq login 1
```

## WSL User

```
$ curl -sSL https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.sh | bash

Detected: WSL (Windows Subsystem for Linux)
✓ claude found
✓ python3 found
✓ jq found
✓ Files installed to ~/.claude/accounts/
✓ csq installed to ~/.local/bin/
  Skipping keychain setup (file-only credential storage on WSL)
✓ Settings configured (statusline + auto-rotate hook)

Run: csq login 1
```

## Windows PowerShell User

```
PS> irm https://raw.githubusercontent.com/terrene-foundation/claude-squad/main/install.ps1 | iex

Detected: Windows (PowerShell)
✓ claude found
✓ python3 found
  jq not needed (PowerShell has built-in JSON)
✓ Files installed to C:\Users\jack\.claude\accounts\
✓ csq.ps1 installed to C:\Users\jack\.claude\accounts\
✓ csq.cmd wrapper created in PATH
  Skipping keychain setup (file-only credential storage on Windows)
✓ Settings configured (statusline + auto-rotate hook)

Run: csq login 1
```

# User Flow: Daily Use (same across platforms)

```
$ csq login 1        # save account 1 (opens browser)
$ csq login 2        # save account 2
$ csq run 1          # terminal 1 on account 1
$ csq run 2          # terminal 2 on account 2

# When rate limited (inside CC):
! csq swap 3         # swap to account 3, same conversation

# Check status:
$ csq status
```

# User Flow: VS Code

VS Code users install csq via the same installer for their platform. The VS Code Claude Code extension reads the same `settings.json` that csq patches, so:

1. Statusline shows account + quota in VS Code's Claude Code panel
2. Auto-rotate hook fires on VS Code interactions
3. `! csq swap N` works in VS Code's Claude Code terminal

No VS Code extension or plugin is needed.
