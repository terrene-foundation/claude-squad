# TODO: Replace bc with awk in statusline

**Milestone**: 0 — Quick Fixes
**File**: `statusline-quota.sh` lines 69-76
**Blocks**: None
**Blocked by**: None

## What

Replace `echo "scale=1; $n / 1000000" | bc` (and similar) with `awk "BEGIN{printf ...}"`.

`bc` is not installed on Alpine Linux, Arch Linux minimal, or many Docker containers. `awk` is universal.

## Current (3 lines)

```bash
if [ "$n" -ge 1000000 ]; then
    printf "%.1fM" "$(echo "scale=1; $n / 1000000" | bc)"
elif [ "$n" -ge 1000 ]; then
    printf "%.0fk" "$(echo "scale=0; $n / 1000" | bc)"
```

## Target

```bash
if [ "$n" -ge 1000000 ]; then
    awk "BEGIN{printf \"%.1fM\", $n/1000000}"
elif [ "$n" -ge 1000 ]; then
    awk "BEGIN{printf \"%.0fk\", $n/1000}"
```

## Acceptance

- `grep -c 'bc' statusline-quota.sh` returns 0
- `bash -n statusline-quota.sh` passes
- Statusline renders token counts correctly (test: context window with 1.2M tokens shows "1.2M")
