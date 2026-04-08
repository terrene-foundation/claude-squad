# TODO: Use junctions for directories on Windows, copy for files

**Milestone**: 2 — Smart Installer
**File**: `csq` (symlink logic in cmd_run, lines ~157-195)
**Blocks**: None
**Blocked by**: Todo 16 (platform detection)

## What

The symlink loop in `cmd_run` creates symlinks from `config-N/` to `~/.claude/` for shared artifacts. On Windows:

- **Directory symlinks** → use junctions (`cmd //c mklink /J` or `ln -s` which Git Bash may support)
- **File symlinks** → copy the file (junctions are directory-only on Windows)

File copies introduce a staleness risk for files CC writes (preferences, history). For most shared files this is acceptable because they change rarely, and `csq run` creates fresh copies on each start.

## Detection

```bash
_create_link() {
    local source="$1" target="$2"
    if [[ "$PLATFORM" == "git-bash" ]]; then
        if [[ -d "$source" ]]; then
            cmd //c "mklink /J \"$(cygpath -w "$target")\" \"$(cygpath -w "$source")\"" 2>/dev/null \
                || cp -r "$source" "$target"
        else
            cp "$source" "$target" 2>/dev/null || true
        fi
    else
        ln -s "$source" "$target"
    fi
}
```

## Acceptance

- On macOS/Linux: symlinks created as before
- On Git Bash Windows: junctions for directories, copies for files
- `csq run N` starts CC successfully on Windows
- Shared state (projects, sessions) visible in config-N directory
