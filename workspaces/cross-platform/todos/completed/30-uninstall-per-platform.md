# TODO: Platform-specific uninstall instructions

**Milestone**: 4 — Documentation
**File**: `README.md`
**Blocks**: None
**Blocked by**: Todo 20 (junction logic)

## What

Windows junctions are a data-loss risk during uninstall. `rm -rf` in Git Bash follows junctions and deletes the TARGET contents (the real `~/.claude/projects/`, `~/.claude/sessions/`, etc.), not just the junction.

Document safe uninstall per platform:

### macOS / Linux / WSL

```bash
rm -rf ~/.claude/accounts
rm ~/.local/bin/csq
# Remove hooks and statusLine from ~/.claude/settings.json
```

### Windows (Git Bash)

```bash
# IMPORTANT: Remove junctions FIRST (do NOT use rm -rf on directories with junctions)
for d in ~/.claude/accounts/config-*/; do
    for item in "$d"*/; do
        [ -L "$item" ] && rm "$item"  # remove symlink/junction only
    done
done
rm -rf ~/.claude/accounts
# Remove hooks and statusLine from ~/.claude/settings.json
```

### Windows (PowerShell)

```powershell
# Remove junctions safely
Get-ChildItem "$env:USERPROFILE\.claude\accounts\config-*" -Directory |
    Get-ChildItem -Attributes ReparsePoint |
    ForEach-Object { $_.Delete() }
Remove-Item -Recurse -Force "$env:USERPROFILE\.claude\accounts"
```

## Acceptance

- Uninstall instructions per platform in README
- Junction warning is prominent
- No data loss when following instructions
