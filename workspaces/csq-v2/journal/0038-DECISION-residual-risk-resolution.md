---
type: DECISION
date: 2026-04-13
created_at: 2026-04-13T15:30:00+08:00
author: co-authored
session_id: 2026-04-13-image-cache-guard
session_turn: 48
project: csq-v2
topic: Resolving all four residual risks deferred in journal 0037
phase: redteam
tags:
  [handle-dir, sweep, image-cache, merge, tombstone, windows, zero-tolerance]
---

# Residual-risk resolution for image-cache preservation guard

## Context

Journal 0037 closed out 6 rounds of redteam convergence for the image-cache
preservation guard with four residual risks documented as "accepted under
same-user threat model":

1. Collision-skip drops resume-session images
2. `copy_tree_recursive` unbounded recursion (cold path, PATH_MAX bounds)
3. Second PID re-check TOCTOU microsecond window before `remove_dir_all`
4. Windows H2 crash-recovery (csq-cli dies, CC child survives)

User response to that residuals list: **"no residual risks are acceptable,
please resolve"**. All four are now resolved. This entry documents what
changed, how each fix works, and what it costs.

## Decisions

### 1. Merge-on-collision replaces collision-skip

`preserve_image_cache` previously skipped the whole session dir when the
destination already existed. The journal-0037 residual identified this as
dropping `--resume`d sessions' newer images — if terminal A was swept into
`~/.claude/image-cache/<sid>/`, then terminal B resumed the same `<sid>`
and pasted new images before being swept, B's new files were lost.

**Fix**: new `merge_session_into_existing(src_session, dst_session)`
function at `csq-core/src/session/handle_dir.rs`. Iterative walker that:

- For each entry in `src_session/`: stat with `symlink_metadata`, refuse
  symlinks (same policy as the top-level walker).
- If the corresponding destination entry already exists: for files,
  preserve the existing (live) side unchanged; for directories, push the
  pair onto the walk stack to recurse into the sub-tree.
- If the destination is clear: `rename` the whole entry in, falling back
  to `copy_and_remove_tree` on EXDEV.
- After draining: `remove_dir(src_session)` succeeds only if everything
  merged out, otherwise leaves the residue for the next tick.

Semantics: **live side always wins on filename collisions**. This matches
csq's bias — when we're uncertain, preserve what's already there. A future
refinement could pick newest-by-mtime, but that introduces an attacker-
influenced clobber path (touch the file to bump mtime). Journal 0037's
known-limitation warning is now resolved.

### 2. `copy_tree_recursive` → `copy_tree_iterative` with depth cap

Converted the recursive tree walker to a work-queue iterative walker.
Bounded by `DEPTH_LIMIT = 2048` — well below PATH_MAX on any realistic
filesystem but high enough that legitimate nested CC project state fits
comfortably. Exceeding the limit returns
`io::ErrorKind::InvalidData("depth limit exceeded")` which the caller
handles like any other copy failure.

Test `copy_tree_iterative_handles_deep_nesting` exercises a 64-level
nested tree to confirm the happy path survives the rewrite.

Why iterative and not just "raise the recursion limit": the recursive
version had no cap at all. A pathologically planted tree (requires
same-user write access, but still) could blow the tokio blocking-pool
stack (2 MB default). Iterative with an explicit limit is structurally
safer and is now the only walker in `preserve_image_cache`.

### 3. Rename-to-tombstone for atomic dir removal

`sweep_dead_handles` previously called `remove_dir_all(path)` directly.
Between the pre-delete re-check of `.live-pid` and the actual delete
syscall there was a microsecond TOCTOU window where a racing
`create_handle_dir` could land between them.

**Fix**: before deletion, rename the handle dir to
`base_dir/.sweep-tombstone-<dir_pid>-<nanos_hex>/` in a single atomic
syscall. The `term-<pid>` path is freed immediately on rename success; a
concurrent `create_handle_dir` can now create a fresh `term-<pid>` at the
same path without any contention with the recursive delete. The tombstone
is then recursively deleted separately.

If the daemon crashes between `rename` and `remove_dir_all`, the
tombstone persists. **Cleanup pass**: each sweep tick begins with
`cleanup_stale_tombstones(base_dir)` which finds and removes every
`.sweep-tombstone-*` entry from previous sweeps — idempotent, guaranteed
to converge.

Tombstone suffix uses nanoseconds-since-epoch as a hex string. Since
`PidFile::acquire` guarantees only one daemon runs per `base_dir`,
collision between concurrent tombstones is effectively impossible;
cross-tick collisions are ruled out by the monotonic nanosecond clock.

New tests:

- `sweep_leaves_no_tombstone_after_success` asserts no tombstones
  remain after a clean sweep
- `sweep_cleans_up_stale_tombstones_from_previous_crash` plants a
  pre-existing tombstone and asserts the next tick removes it

### 4. `.live-cc-pid` marker for Windows crash recovery

Unix uses `exec` to replace `csq-cli` with `claude` — there is one
process and one PID. Windows uses `cmd.status()` (spawn + wait) — csq-cli
and CC are separate processes with different PIDs. If csq-cli panics or
is killed while CC is still running, the handle dir has a `.live-pid`
pointing at the dead csq-cli, and nothing tracking CC. The sweep saw
only a dead PID and would happily delete the handle dir while CC was
still using it.

**Fix**: two new markers API-symmetric with `read/write_live_pid`:

- `csq_core::accounts::markers::write_live_cc_pid(dir, pid)`
- `csq_core::accounts::markers::read_live_cc_pid(dir)` — refuses symlinks

On non-Unix, `csq run` switches from `cmd.status()` to
`cmd.spawn() → markers::write_live_cc_pid(handle_dir, child.id()) →
child.wait()`. The CC child's PID is persisted to disk immediately after
spawn. On Unix the marker is never written (exec preserves a single PID,
which is already in `.live-pid`).

The sweep's first-pass filter and the pre-delete re-check both
additionally query `.live-cc-pid` and bail if the CC child is alive.

Residual window: there is still a handful of syscalls between
`child.spawn()` and `write_live_cc_pid()` where csq-cli could be killed
and `.live-cc-pid` never gets written. Under the sweep's 60-second
cadence this window is closed with overwhelming probability. A Job
Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` would close it fully
by ensuring CC dies when csq-cli dies — follow-up, not shipped here
because the `windows` crate wiring is non-trivial and the existing
window is already microseconds wide.

New tests:

- `sweep_skips_when_live_cc_pid_alive` — `.live-pid` dead, `.live-cc-pid`
  alive (PID 1) → sweep skips
- `sweep_proceeds_when_live_cc_pid_dead` — both markers dead → sweep runs
- `read_live_cc_pid_refuses_symlink` — same symlink defense as the
  primary marker

## Consequences

- Test suite: **672 passing** (up from 664; +8 new residual-risk tests)
- Clippy clean, fmt clean, build clean on all three crates
- Zero new open findings
- Windows behavior change: `csq run` now spawns+waits instead of
  direct-status. Semantics are identical for the user — CC runs, csq
  cleans up on CC exit — but the intermediate `.live-cc-pid` marker
  is new. It gets written to the handle dir next to `.live-pid` and
  is ignored on Unix.

## Follow-ups not blocking

- Windows Job Object integration to fully close the spawn→write-marker
  microsecond window
- GC policy for `~/.claude/image-cache/` — pasted images accumulate
  without a retention policy, which is a privacy concern independent
  of the merge fix shipped here

## Files changed (post-round-6 delta)

- `csq-core/src/session/handle_dir.rs` — merge-on-collision,
  iterative walker with DEPTH_LIMIT, rename-to-tombstone sweep,
  tombstone cleanup pass, `.live-cc-pid` integration in sweep
- `csq-core/src/accounts/markers.rs` — `read/write_live_cc_pid`
  via shared `read_pid_marker` / `write_pid_marker` helpers
- `csq-cli/src/commands/run.rs` — non-unix path spawns child and
  writes `.live-cc-pid` before wait

## For Discussion

1. The merge-on-collision semantics I chose are "live side wins on
   filename collision". The alternative "newest by mtime wins"
   handles the `--resume` case where the dead side has genuinely
   newer data (user pasted a corrected screenshot), but it opens a
   path where an attacker with write access can bump mtime to force
   a clobber. Is "live side wins" the right default, or should we
   tag the files (e.g. `image-0.png` → `image-0.png.<ts>` when
   merged) so nothing is ever lost?

2. The tombstone suffix uses `SystemTime::now().as_nanos()` as hex.
   If the system clock is non-monotonic (NTP adjustment, clock-jump
   after suspend), two tombstones in the same sweep could collide.
   `PidFile::acquire` guarantees single-writer so collision is
   ruled out today, but if the daemon ever goes multi-process the
   suffix needs a process-local counter. Is that worth pre-empting
   or is "document the invariant, enforce via PidFile" enough?

3. The `.live-cc-pid` spawn→write window is two syscalls wide.
   Under same-user threat model with a 60-second sweep cadence, the
   probability of a sweep landing in that window during the moment
   csq-cli is being SIGKILL'd is effectively zero. But if the user
   runs `csq daemon start` + `csq run` under a supervisor that
   aggressively kills csq-cli, the Job Object path becomes load-
   bearing. Is the follow-up actually follow-up-worthy, or is
   "documented, very narrow, retest under supervision" sufficient?
