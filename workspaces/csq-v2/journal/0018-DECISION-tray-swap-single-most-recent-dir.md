---
type: DECISION
date: 2026-04-11
created_at: 2026-04-11T18:35:00+08:00
author: co-authored
session_id: session-2026-04-11c
session_turn: 192
project: csq-v2
topic: Tray quick-swap targets one config dir selected by credentials-file mtime
phase: implement
tags: [desktop, tray, ux, rotation, redteam]
---

# DECISION: Tray quick-swap retargets ONE config dir, chosen by `.credentials.json` mtime

## Context

The first red team pass flagged `handle_tray_event` as running blocking I/O on the UI thread. The obvious fix — `spawn_blocking` — doesn't resolve a deeper question: **which config dir(s) should a tray click retarget?**

First impl: retarget every `config-*` dir whose marker already matches the clicked account. This was a no-op — it just re-fanned credentials already in place.

Second impl: retarget every `config-*` dir regardless of marker. This was **destructive and silent** — a user with 5 concurrent CC sessions on 5 different accounts clicking any tray row would collapse all 5 onto one account with zero confirmation.

## Choice

Retarget **one** dir: the `config-N` whose `.credentials.json` file has the most recent mtime.

## Why `.credentials.json` mtime, not dir mtime

Directory mtime only bumps on child create/delete/rename. CC processes modifying session state in place do NOT bump dir mtime. Credential files, however, are rewritten via `atomic_replace` on every OAuth refresh and login — that write operation touches the specific dir the user is actively using. File mtime on `.credentials.json` is therefore a reliable "this is the session the user last exercised" signal.

## Alternatives considered

1. **Retarget all dirs** — destructive, silent, rejected.
2. **Retarget only dirs already on target account** — no-op, rejected.
3. **Read `CLAUDE_CONFIG_DIR` env var** — unset in GUI-launched Tauri processes (inherited only from parent shell). Dead code in practice.
4. **Ask the daemon which dir is active** — requires daemon coupling we don't want for desktop; also the daemon doesn't track "focused" sessions, only refresh state.
5. **Dir mtime** — rejected per above: wrong signal for the intended question.

## Consequences

- Tray click is safe: worst case it retargets one dir the user didn't expect, not five.
- Dirs without `.credentials.json` are skipped entirely (they're not live sessions).
- Symlinks are rejected via `file_type().is_symlink()` which does not follow — prevents write redirection.
- Config dir name validated as `config-<1..=999>`; `config-abc`, `config-../etc`, and out-of-range numbers are all rejected.
- Result is emitted via `tray-swap-complete` event (`{ account, config_dir, ok, error }`) so the frontend can show a toast.
- Tray clicks are serialized via `SWAP_IN_FLIGHT: AtomicBool` with RAII `ClearFlag` — a second click while one is in flight is dropped with a log line.

## For Discussion

1. If a user has two concurrent live sessions with similar activity, the "most recent credential mtime" signal can pick the wrong one by milliseconds. Should the tray click require an explicit confirmation for multi-session users, or is the current "one-click-one-session-switched" UX strictly better than opening the dashboard?
2. If the credential file mtime is unreliable on HFS+ (1s granularity, now mostly deprecated) or on NFS, is there a fallback signal we should consult — e.g., file access time of `settings.json` or a per-dir tick counter maintained by the daemon?
3. Once M8-03 (Windows named-pipe IPC) lands and the daemon tracks focused sessions via a session registry, should tray clicks delegate to the daemon's "most recent swap destination" instead of mtime-scanning the filesystem?
