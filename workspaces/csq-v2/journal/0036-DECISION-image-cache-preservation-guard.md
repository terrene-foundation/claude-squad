---
type: DECISION
date: 2026-04-13
created_at: 2026-04-13T13:00:00+08:00
author: co-authored
session_id: 2026-04-13-image-cache-guard
session_turn: 14
project: csq-v2
topic: image-cache preservation guard in sweep_dead_handles
phase: implement
tags: [handle-dir, sweep, image-cache, sessions, cleanup-guard]
---

# Preserve per-session image caches in `sweep_dead_handles`

## Context

CC writes pasted images under `$CLAUDE_CONFIG_DIR/image-cache/<session-id>/`
and periodically runs `Dv7()` to delete every entry in `image-cache/` that
doesn't match the live session ID. Journal 0035 worked out why symlinking
`image-cache/` back to `~/.claude/image-cache/` via `SHARED_ITEMS` is unsafe:
concurrent CC processes race to delete each other's active caches because
each sees the shared directory and prunes every session ID that isn't
its own.

The handle-dir model (spec 02) means `$CLAUDE_CONFIG_DIR` is
`term-<pid>`. When the daemon sweeps a dead handle dir via
`sweep_dead_handles`, any pasted images under that dir's
`image-cache/` vanish. A resumed conversation in a new handle dir
then references images that no longer exist on disk.

## Decision

Add a `preserve_image_cache` guard that runs inside `sweep_dead_handles`,
**before** the dead handle dir is deleted. It walks
`term-<pid>/image-cache/<session-id>/` subdirectories and
`std::fs::rename`s each one into `~/.claude/image-cache/<session-id>/`.

Key properties:

1. **Per-session granularity** — moves individual `<session-id>/` dirs,
   never the whole `image-cache/` tree. Session IDs are UUIDs, so
   cross-handle collisions are effectively impossible; if one somehow
   happens, we skip and log rather than clobber an existing dir that
   could belong to a live sibling.
2. **Best-effort** — any rename failure is logged and swallowed.
   Preservation MUST NOT block orphan cleanup. The worst case is "image
   lost" (already the pre-fix behavior), never "handle dir not removed".
3. **Path-sensitive, not symlink-sensitive** — `std::fs::rename` is
   atomic on the same filesystem. Both source and destination live
   under the user's home, so they share a mount in every realistic
   layout.
4. **No reintroduction of the shared-dir race** — we do NOT symlink
   `image-cache/` into live handle dirs. Each live handle dir still
   has its own `term-<pid>/image-cache/`. CC's `Dv7()` only ever
   iterates per-handle-dir storage, so concurrent terminals cannot
   delete each other's entries. Preservation happens exactly once,
   at sweep time, when we know the source handle is dead.

### API change

`sweep_dead_handles(base_dir)` → `sweep_dead_handles(base_dir, claude_home)`.
`spawn_sweep(base_dir, shutdown)` → `spawn_sweep(base_dir, claude_home, shutdown)`.
Both CLI (`csq daemon start`) and desktop (`daemon_supervisor`) callers
updated to resolve `claude_home` (`CLAUDE_HOME` env override, else
`~/.claude`; fall back to `base_dir` if resolution fails — preservation
becomes a no-op but sweeping itself still runs).

## Alternatives considered

1. **Add `image-cache` to `SHARED_ITEMS`**. Rejected in journal 0035
   because of CC's `Dv7()` "delete everything except current session"
   semantics. Concurrent terminals would race to delete each other's
   caches.
2. **Symlink each resumed session's `image-cache/<session-id>/` into the
   new handle dir on `csq run`**. Rejected as chicken-and-egg: `csq run`
   doesn't know which session IDs will be resumed until CC starts reading
   `.claude.json`. Could be done lazily (a watcher that notices
   `image-cache/<new-uuid>/` appearing in a handle dir, then hardlinks
   upward), but that's strictly more moving parts than the sweep guard
   for the same durability benefit.
3. **File an upstream issue asking CC to switch `Dv7()` to mtime-based
   cleanup**. Still valid as a long-term fix. Not a substitute — we
   don't control CC's release cadence and existing CC versions need to
   work today.

## Follow-up this session (zero-tolerance)

Running the full workspace test suite after the guard landed surfaced
two pre-existing failures neither caused nor triggered by the guard;
zero-tolerance requires fixing them, not noting them.

1. **`tick_3p_zai_polls_and_writes_quota` time-bomb**. The Z.AI mock
   response pinned `nextResetTime` to `1776025018977` (2026-04-12
   16:56 UTC). Once real time passed that instant (today is 2026-04-13),
   `quota::clear_expired` ran on load and nulled out the 5-hour
   window, breaking the assertion. Fix: change both `mock_zai_get`
   and `mock_get_combined` to `nextResetTime: 4102444800000`
   (2100-01-01 in ms). Added a comment explaining _why_ — the next
   person tempted to use "plausible today+X hours" values should see
   the warning first.
2. **`refresh_token_passes_correct_url_and_body` regression from
   commit 8a9fdc9**. The JSON-body switch in PR #86 updated production
   code but missed this integration test, which still expected
   `grant_type=refresh_token&refresh_token=...` form-encoded.
   Fix: parse the captured body as JSON and assert `grant_type`,
   `refresh_token`, `client_id`, `scope` fields. Added a comment
   pointing at commit `8a9fdc9` and journal 0034 so a future red
   teamer can trace the invariant.

## Consequences

- Pasted images in dead handle dirs are now preserved at
  `~/.claude/image-cache/<session-id>/` after sweep. A future
  improvement can symlink these back into live handle dirs on
  resume without reintroducing the shared-dir race — each handle
  dir only ever imports the specific session IDs it resumes,
  never the whole tree.
- Test suite grew from 586 → 653. Four new handle-dir tests:
  preserves-entries, skips-on-collision, handles-missing-cache,
  plus the signature update in sweep_removes_dead_handles and
  sweep_ignores_config_dirs.
- Two pre-existing test failures resolved (zai time-bomb, refresh
  JSON body regression). Removes entries from the session-notes
  "flaky test" tracking.

## Files changed

- `csq-core/src/session/handle_dir.rs` — add `preserve_image_cache`,
  wire into `sweep_dead_handles`, update `spawn_sweep` signature,
  four new tests
- `csq-cli/src/commands/daemon.rs` — pass `claude_home` to `spawn_sweep`
- `csq-desktop/src-tauri/src/daemon_supervisor.rs` — same
- `csq-core/src/daemon/usage_poller/third_party.rs` — future-proof
  `nextResetTime` in Z.AI mocks
- `csq-core/tests/credential_integration.rs` — JSON body assertion

## For Discussion

1. Journal 0035 listed symlinking individual session subdirectories
   back into live handle dirs as a follow-up. Given each live handle
   still has its own `image-cache/` and CC's `Dv7()` only prunes
   _within_ that per-handle dir, is there any additional work left
   for image _retrieval_, or is the preservation guard sufficient
   once we pair it with a "on `csq run`, look up any
   `~/.claude/image-cache/<resumed-session-id>/` and hardlink it
   into the new handle dir" step?
2. If the mock reset times had been set with `SystemTime::now() +
Duration::from_days(365)` instead of a hardcoded literal, would the
   zai time-bomb have been caught _earlier_ — at what cost? (Counter:
   time-parameterized test data is reproducible across machines but
   introduces a `now()` dependency; hardcoded 2100-01-01 is dumb and
   simple.)
3. The guard's fallback (`preservation becomes a no-op but sweeping
itself still runs`) triggers when `super::claude_home()` errors out
   — e.g. `dirs::home_dir()` returns `None` or `CLAUDE_HOME` contains
   a non-UTF8 path. In which production environment would we actually
   hit that, and should we treat it as a hard failure instead of
   silently losing preservation? (The counter is: sweeping is more
   valuable than preservation — orphaned `term-*` dirs fill the disk
   indefinitely if sweep is blocked.)
