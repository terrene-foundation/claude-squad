# TODO: Validate full csq workflow under Git Bash on Windows

**Milestone**: 3 — Windows Validation
**File**: CI output / manual test
**Blocks**: Todo 25 (PowerShell decision gate)
**Blocked by**: Todos 22, 23

## What

Run the full csq workflow on windows-latest GitHub Actions runner using Git Bash:

1. `bash install.sh` — completes without error
2. `csq login N` — captures credentials from `.credentials.json` (no browser in CI, so mock the auth step)
3. `csq status` — shows account info from mocked credentials
4. `$PYTHON rotation-engine.py swap N` — swaps credentials between mocked accounts
5. `$PYTHON rotation-engine.py statusline` — returns formatted statusline string
6. Symlinks/junctions — verify shared artifacts are accessible

## Mock Strategy for CI

Can't do browser OAuth in CI. Instead:

1. Write synthetic credentials to `credentials/1.json` and `credentials/2.json`
2. Write synthetic profiles to `profiles.json`
3. Test `swap`, `status`, `statusline`, `cleanup` commands
4. Skip `login` and `run` (require real CC installation)

## Acceptance

- All non-interactive csq commands work under Git Bash on Windows
- rotation-engine.py locking, PID detection, and rename work on Windows
- Results documented in CI output
