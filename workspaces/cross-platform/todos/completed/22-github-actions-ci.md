# TODO: Create GitHub Actions CI matrix

**Milestone**: 3 — Windows Validation
**File**: `.github/workflows/test.yml` (new)
**Blocks**: Todo 23 (smoke tests)
**Blocked by**: Milestones 1 and 2

## What

Create a GitHub Actions workflow that runs on push/PR across 3 platforms:

```yaml
name: Cross-Platform Tests
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - name: Run smoke tests
        shell: bash
        run: bash test-platform.sh
```

On Windows, `shell: bash` uses Git Bash (pre-installed on GitHub Actions Windows runners).

## Acceptance

- CI passes on all 3 platforms
- Failures are visible in PR checks
