# Cross-Platform Support for Claude Squad

## Objective

Make claude-squad work on macOS, Linux native, WSL, and Windows PowerShell. The installer must auto-detect the platform and install the correct variant. VS Code Claude Code plugin users get support automatically via the same hooks system.

## Current State

- All code is macOS-only: bash scripts, macOS Keychain, fcntl, POSIX paths
- rotation-engine.py is Python 3 (mostly portable except keychain + fcntl)
- csq is a ~420-line bash script
- statusline-quota.sh and auto-rotate-hook.sh are bash
- install.sh is bash

## Target Platforms

1. **macOS** — current, fully working
2. **Linux native** — bash works, no macOS Keychain, fcntl works
3. **WSL** — same as Linux but config paths may differ, VS Code integration
4. **Windows PowerShell** — needs PowerShell ports of all bash scripts, Windows Credential Manager, different file locking, different config paths

## Requirements

- Smart installer detects platform automatically
- Single rotation-engine.py works on all platforms (platform branches for keychain, locking, paths)
- Bash scripts serve macOS + Linux + WSL
- PowerShell scripts serve Windows native
- VS Code plugin users need no additional setup — hooks in settings.json work across all platforms
- Credential storage: macOS Keychain / Windows Credential Manager / file-only (Linux/WSL)
- README documents all platforms

## Constraints

- No new Python dependencies for core (stdlib only for rotation-engine.py)
- PowerShell 5.1+ (ships with Windows 10/11)
- Must not break existing macOS installations
