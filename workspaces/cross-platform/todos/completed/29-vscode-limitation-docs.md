# TODO: Document VS Code limitations

**Milestone**: 4 — Documentation
**File**: `README.md`
**Blocks**: None
**Blocked by**: Milestone 3

## What

Add a VS Code section to README explaining:

1. **No plugin needed** — VS Code Claude Code extension reads the same settings.json
2. **Hooks are unreliable in VS Code** — known CC bugs (#18547, #16114, #28774):
   - Statusline may not render in VS Code panel
   - Auto-rotate hook may not fire
3. **Core swap works regardless** — `! csq swap N` is a shell command, works in VS Code's integrated terminal
4. **Recommendation**: For full csq features (statusline, auto-rotate), use CLI terminals. VS Code is supported for manual swap.

## Acceptance

- VS Code section in README
- No false promises about hook reliability
- Clear that core functionality (swap) works everywhere
