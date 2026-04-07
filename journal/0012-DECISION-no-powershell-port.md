---
type: DECISION
date: 2026-04-07
created_at: 2026-04-07T22:40:00+08:00
author: co-authored
session_id: 47ffd8ee-c58b-48c9-8e06-2c8ffcfc0d7a
session_turn: 60
project: claude-squad
topic: Skip PowerShell port — bash scripts work on Windows via Git Bash
phase: implement
tags: [cross-platform, windows, scope, ci]
---

## Decision

No PowerShell port of csq, statusline-quota.sh, auto-rotate-hook.sh, or install.sh. Windows users run the same bash scripts via Git Bash (which Claude Code already requires). The cross-platform support shipped is: macOS, Linux, WSL, and Windows-via-Git-Bash, all with the same bash scripts.

## Rationale

Initial plan called for ~400 lines of PowerShell ports as a parallel script set. Two red team passes both flagged the same insight: **CC requires Git for Windows, which ships Git Bash. Every Windows CC user already has bash.** PowerShell ports would be net duplication with parallel maintenance burden.

We tested this empirically. After making the bash scripts platform-aware (`find_python` to handle `python` vs `python3`, junction creation via `mklink /J` for Git Bash, chmod skipped on NTFS, file-only credential storage on non-macOS), we ran `test-platform.sh` on `windows-latest` in GitHub Actions. **20/20 PASS**, including the Windows-specific Python paths (`ctypes.windll.kernel32` for PID detection, `CreateToolhelp32Snapshot` for process tree, `CreateMutexW` for file locking, `os.replace` retry for NTFS file-in-use).

Decision gate criteria:

- All install.sh + csq + hooks + tests work in Git Bash → **NO PowerShell needed** ✓
- Hooks don't fire from Git Bash → write .ps1 hook scripts only ✗
- Symlinks/junctions fail in Git Bash → add junction logic to csq ✗
- python3 not found despite find_python → fix detection ✗

All criteria resolved to "Git Bash works".

## Consequences

- **~50% less code to maintain.** No parallel PowerShell scripts.
- **One installer for everyone.** `install.sh` detects platform internally and adjusts behavior. Same `curl ... | bash` command works everywhere (in Git Bash on Windows).
- **Documentation is simpler.** README has one install command, one usage section, one set of file paths. Platform differences are noted in a Requirements table.
- **Native Windows users without Git Bash are unsupported.** This is fine because CC requires Git for Windows.
- **VS Code integration is automatic.** The VS Code Claude Code extension reads the same `~/.claude/settings.json`. No plugin or extra setup. (With known caveats about hook reliability — see README troubleshooting.)

## Alternatives Considered

1. **Full PowerShell port (rejected)** — initial plan. Would have been ~400 lines of duplicate code with no clear user benefit. Anyone who has CC on Windows already has Git Bash.
2. **Hybrid: PowerShell statusline only** — considered because PowerShell `ConvertFrom-Json` is faster than bash+jq. Rejected because the time savings (sub-millisecond) are dwarfed by the python3 invocation cost in the same script.
3. **Native Windows installer (.msi)** — out of scope for a single-author tool. The bash installer with curl pipe is the standard for developer CLI tools.

## Evidence

- GitHub Actions run 24083444968 — `cross-platform/.github/workflows/test.yml` matrix
- Matrix: macos-latest, ubuntu-latest, windows-latest × Python 3.11, 3.12 (6 jobs)
- All 6 jobs PASS, all 20 test cases per job

## For Discussion

1. The bash scripts are now genuinely cross-platform via the `_find_python` and `PLATFORM` helpers. Is this a pattern other "Mac developer tool" projects could adopt to support Windows without writing PowerShell? What's the boundary where it stops being practical?
2. Git Bash on Windows is a strong dependency. If a future version of CC drops the Git for Windows requirement, csq's Windows support breaks too. How do we monitor for that?
3. The CI matrix tests Git Bash but doesn't test PowerShell-native operation. If a Windows user explicitly tries `powershell.exe install.ps1`, they get nothing — not even a clear error message redirecting them to Git Bash. Should the installer at least detect this and print a helpful message?
