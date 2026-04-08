# TODO: Create cross-platform smoke test script

**Milestone**: 3 — Windows Validation
**File**: `test-platform.sh` (new)
**Blocks**: Todo 22 (CI matrix calls this script)
**Blocked by**: Milestones 1 and 2

## What

Bash script that validates core csq functionality per platform. Runs in CI and can be run manually.

## Test Cases

1. **Python detection (bash)**: `find_python` returns a working Python 3 command
2. **Python detection (engine)**: `$PYTHON rotation-engine.py python-cmd` returns valid command
3. **Engine syntax**: `$PYTHON rotation-engine.py --help` exits 0
4. **Engine status**: `$PYTHON rotation-engine.py status` runs without crash (may show "no accounts")
5. **File locking**: Two concurrent `$PYTHON rotation-engine.py update '{"rate_limits":{}}' &` don't corrupt quota.json
6. **Atomic rename**: Write temp file, `os.replace()` over existing target, verify content
7. **Platform detection**: `detect_platform` returns expected value for the current OS
8. **Credential file write/read**: Write test JSON to credentials dir, read back, verify
9. **Symlink/junction creation**: Create link, verify target accessible (directory junction on Windows)
10. **Junction cleanup safety**: Remove junction, verify target directory NOT deleted
11. **PID detection**: `_is_pid_alive(current_pid)` returns True, `_is_pid_alive(99999)` returns False
12. **Process tree**: `_find_cc_pid()` returns None when CC not running (no crash on any platform)
13. **Shebang check**: All .sh files use `#!/usr/bin/env bash`
14. **No hardcoded python3**: `grep -c 'python3' csq statusline-quota.sh auto-rotate-hook.sh` only in find_python()

## Acceptance

- `bash test-platform.sh` exits 0 on macOS, Linux, Windows (Git Bash)
- Each test case prints PASS/FAIL with clear description
- Failed tests print diagnostic info (platform, Python version, error message)
