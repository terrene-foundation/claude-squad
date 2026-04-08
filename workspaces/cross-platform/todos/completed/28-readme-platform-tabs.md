# TODO: Update README with platform-specific install instructions

**Milestone**: 4 — Documentation
**File**: `README.md`
**Blocks**: None
**Blocked by**: Milestone 3

## What

Add platform sections to README:

### Install section

- macOS / Linux / WSL: `curl -sSL ... | bash`
- Windows (Git Bash): same curl command (Git Bash has curl)
- Windows (PowerShell): `irm ... | iex` (only if Todo 26 produced install.ps1)

### Requirements section

Update to list per-platform requirements:

- macOS: Python 3, jq (optional), Claude Code CLI
- Linux/WSL: Python 3, jq (optional), Claude Code CLI
- Windows: Python 3, Claude Code CLI (includes Git Bash), jq not needed with PowerShell hooks

### Troubleshooting section

- Windows symlink permissions → enable Developer Mode
- WSL credential sharing → file-only is normal, not an error
- VS Code hooks may not fire → known CC limitation, core swap works regardless
- `python3` not found on Windows → ensure Python 3 is on PATH, or use `py` launcher

### Uninstall section

Platform-specific instructions, especially junction cleanup on Windows:

- macOS/Linux: `rm -rf ~/.claude/accounts && rm ~/.local/bin/csq`
- Windows: remove junctions FIRST with `rmdir`, then remove accounts directory

## Acceptance

- README covers all 4 platforms
- Install instructions work when followed literally
- Troubleshooting section addresses known platform issues
