---
kind: DISCOVERY
date: 2026-04-13
---

# CC `image-cache` is per-CLAUDE_CONFIG_DIR with a racey cleanup

Decoded from the minified `cli.js`: CC writes pasted images to
`$CLAUDE_CONFIG_DIR/image-cache/<session-id>/`, and periodically
runs a cleanup (`Dv7()`) that removes every entry in
`image-cache/` **except** the current session's ID.

## The handle-dir problem

In the handle-dir model, `$CLAUDE_CONFIG_DIR` is `term-<pid>`.
So images are written to `term-<pid>/image-cache/`. When the
daemon sweeps the dead handle dir, the images vanish. A resumed
conversation from a new handle dir references images that no
longer exist on disk.

## Why a simple symlink fix is wrong

If `image-cache` is added to `SHARED_ITEMS` (all handle dirs
point at `~/.claude/image-cache/`), each terminal's `Dv7()`
cleanup iterates the shared directory and removes every entry
that doesn't match its own current session ID. Concurrent
terminals race to delete each other's active image caches.

## Correct fix (medium term)

Write a csq cleanup guard that runs alongside the daemon's
`sweep_dead_handles`. Before removing a dead handle dir, walk
its `image-cache/<session-id>/` entries and move them into
`~/.claude/image-cache/<session-id>/`. Individual session
subdirectories (not the whole cache) can then be symlinked
back into active handle dirs as needed, or the session UUID
can be looked up directly in the shared location.

Alternative: file an upstream issue asking CC's `Dv7` to use
mtime-based cleanup instead of "everything except current
session."

## Short term

Documented limitation: images pasted into handle-dir terminals
are lost when the handle dir is swept. Tracked for next session.
