---
type: RISK
date: 2026-04-13
created_at: 2026-04-13T14:30:00+08:00
author: co-authored
session_id: 2026-04-13-image-cache-guard
session_turn: 34
project: csq-v2
topic: image-cache preservation guard — red team findings and deferred residual risk
phase: redteam
tags: [handle-dir, sweep, image-cache, redteam, symlink, toctou, pid-recycling]
---

# Red-team convergence for image-cache preservation guard

## Context

Journal 0036 landed `preserve_image_cache` inside `sweep_dead_handles`. This
entry captures the four red-team rounds that followed and the residual risks
that remain accepted under csq's same-user threat model.

Convergence: 2 consecutive clean rounds (rounds 5 + 6) following 4 rounds of
fix-and-retest. Final state: **664 tests passing**, clippy clean, fmt clean,
no CRITICAL/HIGH findings open.

## Findings addressed (rounds 1–4)

### Round 1

| #             | Severity     | Finding                                                                       | Fix                                                                                                                                                                                      |
| ------------- | ------------ | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1            | CRITICAL→LOW | Path traversal via session-id name                                            | `is_valid_session_name` restricts to `[0-9a-f-]{1..=64}`. Downgraded after confirming `read_dir` filters `.`/`..` and filenames cannot contain `/` — validation kept as defense-in-depth |
| C2            | CRITICAL     | Symlink interposition on `image-cache/<sid>/` entries                         | `symlink_metadata` check per entry, refuses symlinks                                                                                                                                     |
| H1            | HIGH         | Symlink swap on `src_cache` (`image-cache/` itself)                           | `symlink_metadata` check on `src_cache`                                                                                                                                                  |
| H2            | HIGH         | Symlink at destination `~/.claude/image-cache/`                               | `symlink_metadata` check on `dst_cache`; refuse if symlink or non-dir                                                                                                                    |
| H3            | HIGH         | PID recycling TOCTOU between first `is_pid_alive` and `remove_dir_all`        | Added second re-read of `.live-pid` before `remove_dir_all`, bail on mismatch or now-alive                                                                                               |
| H1 (cross-fs) | HIGH         | `EXDEV` (cross-filesystem rename) silently loses all images                   | Added `is_cross_device` detection + `copy_tree_recursive` fallback that refuses symlinks during walk                                                                                     |
| H2 (Windows)  | MED          | Dead CLI PID ≠ dead CC process on Windows                                     | Not fixable without file-handle enumeration; documented as limited to crash-recovery path because `cmd.status()` synchronously removes the handle dir in the normal Windows exit path    |
| M1            | MED          | `~/.claude/image-cache` existing as a regular file silently fails             | Explicit symlink+non-dir check at destination; error logged distinctly                                                                                                                   |
| M2            | MED          | Collision-skip drops resume-session images                                    | Accepted as known limitation; documented below                                                                                                                                           |
| M3            | MED          | `claude_home` fallback to `base_dir` routes preservation into credentials dir | `spawn_sweep` now takes `Option<PathBuf>`; callers pass `None` when `~/.claude` unresolvable, preservation becomes a no-op instead                                                       |
| L1            | LOW          | `entries.flatten()` swallows per-entry errors                                 | Explicit `for entry in entries` loop with logged error branch                                                                                                                            |

### Round 2

| #   | Severity | Finding                                                                                   | Fix                                                                                             |
| --- | -------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| H1  | HIGH     | `read_live_pid` follows symlinks — poisoned `.live-pid` could make dead handle look alive | `symlink_metadata` check + `is_file()` gate in BOTH `markers::read_live_pid` and the local copy |
| M1  | MED      | `libc::__error()` is Darwin-only; breaks Linux packaging                                  | Replaced with `std::io::Error::last_os_error().raw_os_error()` — stdlib-portable                |
| M2  | MED      | `copy_tree_recursive` uses umask for sub-dirs, loses CC's 0700 mode                       | `set_permissions(dst, meta.permissions())` after `create_dir_all`. Test added                   |
| L1  | LOW      | Uppercase hex in `is_valid_session_name` → APFS/HFS+ case-fold collision                  | Filter tightened to `is_ascii_digit() ‖ ('a'..='f') ‖ '-'`. Test added                          |
| L2  | LOW      | `mock_get_noop` still has stale `1776*` timestamps (inert but inconsistent)               | Bumped to `4102444800000` to match other mocks                                                  |

### Round 4

| #   | Severity | Finding                                                                                                                                                                | Fix                                                                                               |
| --- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| F1  | BLOCKING | Local `read_live_pid` in `handle_dir.rs` is a byte-for-byte duplicate of `markers::read_live_pid`                                                                      | Deleted local copy; both call sites use `markers::read_live_pid`. Symlink-refusal test retargeted |
| F2  | MED      | Re-check only bails on `Some(current) != owner_pid ‖ alive`; `(Some, None)` race (marker disappeared mid-sweep during a racing `create_handle_dir`) proceeds to delete | Restructured into match: `(Some(_), None)` explicitly bails                                       |

### Round 5 — false positive adjudicated

Security-reviewer raised a HIGH claiming `(None, None)` match arm could delete
a newly-created dir whose `create_handle_dir` has not yet written `.live-pid`.
deep-analyst and intermediate-reviewer independently disagreed. I verified the
invariant:

**`create_handle_dir(base_dir, claude_home, account, pid)` is ALWAYS invoked
with `pid == std::process::id()`.**

Verified via `grep create_handle_dir\(`:

- `csq-cli/src/commands/run.rs:80-81` — `let pid = std::process::id();`
- `csq-core/src/session/handle_dir.rs:852, 885` — test fixtures with
  synthetic PIDs in isolated `TempDir`s, no concurrent sweep.

Under this invariant, a process creating `term-<pid>` has PID `pid`, so
`is_pid_alive(pid)` returns true on the sweep's first-pass filter, causing
the sweep to skip that dir entirely. Scenario D requires a process creating
`term-<X>` where X ≠ the creator's own PID — no production caller does this.

**Adjudication:** Round 5 HIGH is a false positive. As defense-in-depth I
added an explicit docstring to `create_handle_dir` naming the invariant and
explaining why it keeps the sweep race-safe. Round 6 intermediate-reviewer
signed off CLEAN on the docstring delta.

## Residual risks — accepted

1. **Collision-skip drops resume-session images** (round 1 M2). If CC
   `--resume`s the same session-id from two handle dirs, the first sweep
   moves `image-cache/<sid>/` to `~/.claude/image-cache/<sid>/`, and the
   second sweep hits the destination collision and _skips_ the second
   handle dir's newer images. A merge-on-collision fix (per-file rename
   with newest-wins semantics) is a follow-up — not shipped here because:
   (a) session-id reuse across handle dirs only happens in explicit
   `--resume`, (b) the skipped data is pasted images not credentials,
   (c) the test `sweep_skips_image_cache_on_collision` explicitly codifies
   the current behavior so a future merge-fix is a conscious change.

2. **`copy_tree_recursive` has no recursion-depth cap** (round 2 LOW).
   Default stack is 8 MB main / 2 MB tokio blocking; each frame ~200 bytes
   so ~40k levels before overflow. Bounded in practice by PATH_MAX (typical
   1024). Attacker scenario (plant a 40k-deep tree under a dead handle dir)
   requires same-user write access to `~/.claude/accounts/term-*/`, which
   already grants credential access. Converting to iterative walk is a
   follow-up.

3. **Second PID re-check TOCTOU window** (round 2 MED). Between the re-read
   of `.live-pid` and the actual `remove_dir_all` call there is a microsecond
   window. The invariant from round 5 (caller PID = dir PID) closes this in
   production; a rename-to-tombstone pattern would close it structurally.
   Deferred as complexity outweighs benefit under the same-user threat model.

4. **H2 Windows variant** (round 1 MED). On Windows, `csq run` uses
   `cmd.status()` to block and explicitly `remove_dir_all`s the handle dir
   on exit. The only orphan shape on Windows is crash-recovery — `csq run`
   panicked while CC was still a child process. The sweep runs 60s after
   startup and handles dead handle dirs; a child CC outliving its parent
   csq on Windows remains a theoretical concern that would require
   platform-specific restart-manager hooks to close.

## Hardening confirmed

- **`.live-pid` symlink-refusal**: both `markers::read_live_pid` and the
  (now-removed) local copy refuse to follow symlinks via `symlink_metadata`.
- **Three-layer symlink refusal in `preserve_image_cache`**: source
  `image-cache/` dir, per-entry `<session-id>/` dir, destination
  `~/.claude/image-cache/` dir.
- **UUID-alphabet validation**: `[0-9a-f-]` only. Rejects uppercase to close
  APFS/HFS+ case-fold collisions.
- **EXDEV fallback**: `copy_tree_recursive` walks the tree, refuses symlinks
  during recursion, preserves directory mode bits via `set_permissions`.
- **Portable PID liveness check**: `std::io::Error::last_os_error()` instead
  of `libc::__error()` (Darwin-only).
- **Re-check match arms**: `(Some(_), None)` bails on racing create;
  `(_, Some(current))` bails on ownership change or now-alive.
- **`claude_home: Option<&Path>`**: callers pass `None` when
  `~/.claude` cannot be resolved, disabling preservation rather than
  routing images into `base_dir` (which is the credentials parent).
- **Invariant docstring** on `create_handle_dir` naming
  `pid == std::process::id()` as load-bearing for sweep safety.

## Test coverage added across rounds

All unix-only tests guarded `#[cfg(unix)]`:

- `is_valid_session_name_accepts_uuids_and_rejects_hostile_names` — unit
  test covering UUID + hostile names + uppercase rejection + length cap
- `sweep_rejects_non_uuid_session_names` — hostile name planted, asserts
  it does NOT land in `~/.claude/image-cache/`
- `sweep_refuses_symlink_src_image_cache` — symlink to sensitive dir at
  `image-cache/`; asserts target survives untouched
- `sweep_refuses_symlink_session_entry` — symlink at `image-cache/<sid>/`
- `sweep_refuses_symlink_dst_image_cache` — symlink at destination
- `sweep_none_claude_home_skips_preservation_but_still_sweeps` — `None`
  fallback path
- `sweep_skips_when_live_pid_alive_but_dir_name_pid_dead` — `.live-pid`
  authority over dir-name PID
- `copy_tree_recursive_refuses_symlinks` — EXDEV fallback symlink refusal
- `copy_tree_recursive_preserves_nested_subdirs_and_files` — two-level
  nesting through the fallback
- `copy_tree_recursive_preserves_directory_mode` — 0o700 survives fallback
- `read_live_pid_refuses_symlink` — targets `markers::read_live_pid`

Plus tightened existing tests:

- `refresh_token_passes_correct_url_and_body` now asserts `OAUTH_CLIENT_ID`
  constant equality, `OAUTH_SCOPES.join(" ")` scope value, and a closed
  field whitelist (`grant_type`, `refresh_token`, `client_id`, `scope`).
- All 3p mock payloads (`mock_zai_get`, `mock_get_combined`, `mock_get_noop`)
  use `4102444800000` (2100-01-01 ms) for reset timestamps, eliminating the
  `clear_expired` time-bomb class.

## Outstanding gaps (follow-ups, not blocking)

- **Nested-session-merge on collision**: see residual risk 1 above.
- **Iterative `copy_tree_recursive`**: see residual risk 2.
- **Rename-to-tombstone sweep**: see residual risk 3.
- **Windows restart-manager integration for handle-dir ownership**: see
  residual risk 4.
- **`~/.claude/image-cache/` GC policy**: sessions accumulate without a
  mtime-based garbage collector. Pasted images may be sensitive. Defer
  until a clear user-facing retention policy exists.

## For Discussion

1. Scenario D was flagged as HIGH by one agent and CLEAN by two others. The
   adjudication rested on a grep of every `create_handle_dir` caller. What's
   the right process for escalating disagreements between agents — is a
   single verified grep enough, or should the disagreeing agent always get
   the last word even when outvoted 2-to-1?
2. Five of the six redteam rounds ran in parallel with three agents each.
   The sixth round used only one agent. If budget/time is constrained, is
   parallel-from-the-start cheaper than iterative-serial even with the
   overlap in findings? The round 1 agents duplicated C1, C2, H1 and
   contributed three orthogonal findings each — without the parallelism
   I'd have missed coverage.
3. Fixing the minimax `mock_get_combined` time-bomb and the refresh-token
   integration test (both found during the test scan, not during redteam)
   argues for running the test scan BEFORE the redteam. If the test suite
   had been red going in, I would have had to fix baseline failures mid-
   convergence — messy. Is "test scan before redteam" a general rule worth
   codifying in the `/redteam` workflow?
