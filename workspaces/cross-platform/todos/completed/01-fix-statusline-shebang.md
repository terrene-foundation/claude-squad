# TODO: Fix statusline shebang

**Milestone**: 0 — Quick Fixes
**File**: `statusline-quota.sh` line 1
**Blocks**: None
**Blocked by**: None

## What

Change `#!/bin/bash` to `#!/usr/bin/env bash`.

## Why

`/bin/bash` doesn't exist on NixOS, Guix, and some minimal Linux distros. The other scripts already use `#!/usr/bin/env bash`. This inconsistency means statusline breaks on those distros while csq works.

## Acceptance

- `head -1 statusline-quota.sh` shows `#!/usr/bin/env bash`
- `bash -n statusline-quota.sh` passes
