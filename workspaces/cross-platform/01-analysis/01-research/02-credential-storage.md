# Credential Storage Per Platform

## How Claude Code Stores Credentials

| Platform                | Primary Storage                                   | Fallback            | Config Dir               |
| ----------------------- | ------------------------------------------------- | ------------------- | ------------------------ |
| macOS                   | macOS Keychain (`Claude Code-credentials-{hash}`) | `.credentials.json` | `~/.claude/`             |
| Linux                   | `.credentials.json` only                          | None                | `~/.claude/`             |
| WSL                     | `.credentials.json` only                          | None                | `~/.claude/`             |
| Windows (Git Bash/MSYS) | Windows Credential Manager (partial — see bug)    | `.credentials.json` | `%USERPROFILE%\.claude\` |
| Windows (PowerShell)    | Windows Credential Manager                        | `.credentials.json` | `%USERPROFILE%\.claude\` |

## Known Issues

### Windows Credential Manager Bug (CC Issue #29049)

On Windows with MSYS/Git Bash, OAuth tokens are NOT persisted to Windows Credential Manager. `cmdkey /list` shows no Claude entries. Users must re-authenticate every new terminal. The `.credentials.json` file is also not created in some cases.

This means **file-only credential storage is the safest default on Windows/WSL/Linux**.

### VS Code Extension Auth Loop (CC Issue #33122)

VS Code extension on Windows can enter an OAuth auth loop — repeatedly loses authentication despite successful browser authorization.

## csq Credential Strategy Per Platform

| Platform | Read Creds               | Write Creds     | Notes                      |
| -------- | ------------------------ | --------------- | -------------------------- |
| macOS    | Keychain → file fallback | Keychain + file | Current behavior, keep     |
| Linux    | File only                | File only       | Skip all keychain code     |
| WSL      | File only                | File only       | Same as Linux              |
| Windows  | File only                | File only       | Safest given CC bug #29049 |

**Decision: use file-only (`credentials/N.json` + `config-N/.credentials.json`) on all non-macOS platforms.** This is the path that already works. The keychain is a nice-to-have optimization on macOS (survives credential file deletion) but is not essential.

## No New Dependencies

Originally considered `keyring` Python library for cross-platform credential store. Rejected because:

1. File-only already works (CC reads `.credentials.json` on all platforms)
2. Windows Credential Manager has known CC bugs — adding another layer would compound them
3. `keyring` adds a pip dependency to what is currently a zero-dependency Python script
4. The keychain on macOS is best-effort anyway (failures are non-fatal in current code)
