---
type: DECISION
date: 2026-04-12
created_at: 2026-04-12T22:15:00+08:00
author: co-authored
session_id: session-2026-04-12c
session_turn: 380
project: csq-v2
topic: Retract journal 0029 Finding 4; adopt per-account config-N + per-terminal term-<pid> handle-dir model
phase: analyze
tags: [architecture, credentials, keychain, retraction, cc-source, spec]
---

# 0031 — Retract "Stale Session Detection"; Adopt Handle-Dir Model

## Status

**RETRACTS:** Journal 0029 Finding 4 (Stale Session Detection / `needs_restart`) and every code artifact it spawned. **SUPERSEDES:** the `config-N = slot` model that ran from csq-v2 M1 through 2026-04-12. **CODIFIED IN:** `specs/01-cc-credential-architecture.md`, `specs/02-csq-handle-dir-model.md`.

## Context

During a live debugging session on 2026-04-12, the user reported that running `csq swap 2` in one of three terminals on account 3 moved ALL three terminals to account 2, not just the one they ran swap in. The dashboard confirmed the swap, but the user's expectation was per-terminal independence.

Initial diagnosis (mine) proposed that per-terminal swap required per-terminal config dirs — which the user rejected, pointing out vanilla CC does not create per-terminal dirs. The user then described a specific vanilla-CC behavior: "3 terminals on account 1; I `/login` in one to account 2; the other 2 stay on account 1." This contradicted what I had been asserting based on journal 0029 Finding 4.

The user directed me to read CC's actual source at `~/repos/contrib/claude-code-source-code/` rather than rely on what csq journals claimed. The source read produced findings that invalidate Finding 4 and unlock the correct architecture.

## The retraction

**Journal 0029 Finding 4 said:**

> "CC caches credentials in memory at startup. After a swap, the on-disk state changes but the running CC process retains old tokens. Detection heuristic: `.csq-account` marker mtime > process `started_at`."

**CC source says:** `src/utils/auth.ts:1313-1336` implements `invalidateOAuthCacheIfDiskChanged()`, which is called from `checkAndRefreshOAuthTokenIfNeededImpl()` at `auth.ts:1453`, which is called from `services/api/client.ts:132` inside `getAnthropicClient()`. Before every new Anthropic client is built, CC stats `<CLAUDE_CONFIG_DIR>/.credentials.json`. If the mtime differs from the last-seen value, CC clears its in-memory OAuth token memoize and re-reads the file.

The function's comment explicitly describes the two-terminal case:

> "Cross-process staleness: another CC instance may write fresh tokens to disk (refresh or /login), but this process's memoize caches forever. Without this, terminal 1's /login fixes terminal 1; terminal 2's /login then revokes terminal 1 server-side, and terminal 1's memoize never re-reads — infinite /login regress (CC-1096, GH#24317)."

**This function was added by Anthropic specifically to solve the problem Finding 4 claimed was unsolvable.** CC picks up credential changes from any external process on its next API call. Swap is in-flight; there is no restart requirement; the `needs_restart` badge was based on an empirically incorrect hypothesis.

### Evidence chain

1. `src/utils/envUtils.ts:5-13` — `getClaudeConfigHomeDir` is memoized on `CLAUDE_CONFIG_DIR` env var. Cannot be changed mid-process.
2. `src/utils/secureStorage/plainTextStorage.ts:13-17` — credentials file is `<CLAUDE_CONFIG_DIR>/.credentials.json`.
3. `src/utils/auth.ts:1313-1336` — cross-process mtime invalidation. **The load-bearing fact.**
4. `src/utils/auth.ts:1447-1453` — `invalidateOAuthCacheIfDiskChanged` runs inside `checkAndRefreshOAuthTokenIfNeededImpl`.
5. `src/services/api/client.ts:131-133` — `checkAndRefreshOAuthTokenIfNeeded` is called inside `getAnthropicClient()` before every API client construction.
6. `src/utils/secureStorage/macOsKeychainHelpers.ts:29-41` — keychain service name derivation via `sha256(CLAUDE_CONFIG_DIR)[:8]`. Confirms per-directory (not per-terminal) keychain isolation.
7. `src/utils/secureStorage/fallbackStorage.ts:27-62` — macOS `update()` path writes keychain primary, skips file fallback on success.
8. `src/utils/auth.ts:1194-1253` — `saveOAuthTokensIfNeeded` preserves subscription metadata on null fields, mirrors csq's journal 0029 Finding 2 guard.
9. `src/utils/secureStorage/macOsKeychainHelpers.ts:69` — `KEYCHAIN_CACHE_TTL_MS = 30_000`. Bounds cross-process staleness for pure-keychain writes.

All nine are reproducible by grep of the CC source tree at version 2.1.104.

## What this unlocks: the handle-dir model

Per-terminal swap was "physically impossible with shared config dirs" in my earlier message — that was correct, but I framed the fix as "per-terminal config dirs" and the user rejected it because settings.json would duplicate. The correct resolution is separation: **permanent per-account `config-N` dirs own all account state (credentials, settings); ephemeral per-terminal `term-<pid>` handle dirs exist only as symlink wrappers.**

A handle dir's `.credentials.json` is a symlink to the current account's `config-<N>/.credentials.json`. On swap, csq atomically re-points the handle dir's symlinks to a different `config-<M>/*`. CC's `fs.stat` on the symlinked path follows it to the new target, sees a different mtime from its last-seen value (a different file entirely), clears its cache, and the next API call uses account M.

Other terminals (other `term-<otherPid>` dirs) still have their symlinks pointing at the original `config-<current>`. Their stat sees no change. They stay on their current account. True per-terminal swap without settings duplication.

This is documented in full in `specs/02-csq-handle-dir-model.md`. The spec takes precedence over any prior rule, journal, or todo that described a `config-N = slot` model.

## Artifacts to delete

Code artifacts that exist solely because of Finding 4:

1. **`csq-desktop/src-tauri/src/commands.rs:247-251, 295-318`** — the `needs_restart: bool` field on `SessionView` and the marker-mtime-vs-process-start detection heuristic. Both gone.
2. **Journal 0030 item #3** — "needs_restart false positives — csq run N writes marker before spawning claude, so marker is always newer than process. Fixed with 5-second grace period." The "fix" is removed along with the bug it patched.
3. **`csq-desktop/src/lib/components/SessionList.svelte`** (or wherever it lives) — the "restart needed" badge rendering for session rows.
4. **`.claude/rules/account-terminal-separation.md` rule 7** — the "Stale Session Detection After Swap" section with its `marker_mtime > process_start_time` logic. The entire rule becomes obsolete with the handle-dir model.
5. **Journal 0029 Finding 4 section** — left in place as historical record, but marked RETRACTED with a link back to this entry.

Code artifacts that SIMPLIFY but do not disappear:

6. **`rotation::swap_to`** in `csq-core/src/rotation/swap.rs` — becomes a symlink repoint in the handle dir. No longer writes to `config-<N>/.credentials.json`. The subscription-metadata preservation guard is no longer needed in swap (swap doesn't write credentials), but remains needed in the daemon refresh path.
7. **`broker::fanout::fan_out_credentials`** in `csq-core/src/broker/fanout.rs` — collapses dramatically. The daemon writes to `config-<N>/.credentials.json` once; every handle dir with a symlink pointing there automatically follows. The explicit iteration over `config-*` dirs matching a marker is obsolete — there is exactly ONE permanent dir per account by design.
8. **`gap-resolutions.md:635`** — "Auto-rotation is per-terminal" becomes architecturally feasible again. It was described there correctly; the csq code drifted from the description. With handle dirs the code matches the description.

## Alternatives considered (briefly, because most are losing options from prior turns)

1. **Keep per-account config-N without handle dirs, accept that swap affects all terminals on that config dir.** Rejected by the user — they want per-terminal independence for load balancing (move 2 of 3 terminals off a hot account).
2. **Per-terminal config-N dirs (no handle layer), duplicate settings.json per terminal.** Rejected by the user — configuring settings 1000 times is not viable and vanilla CC doesn't force you to.
3. **Keychain-only writes on swap (match vanilla CC /login write path).** Rejected on analysis: csq is a separate process from the running `claude`, so it cannot clear CC's in-process memoize the way CC's own `/login` does. The only cross-process signal CC honors is the file mtime on `.credentials.json`, which means csq MUST write through a file path. A keychain-only write would leave the current terminal stuck on the old account until 401 or token expiry.
4. **Hybrid: csq writes to keychain AND deletes the file to trigger ENOENT path.** Partially works but drops the convergence window to 30 seconds (the keychain TTL) for ALL terminals on the config dir, still no true per-terminal independence. Not a step forward.

The handle-dir model is the only option that satisfies all user-stated constraints (1000 terminals, one login, shared settings, per-terminal swap, in-flight swap) without contradicting CC's actual behavior.

## Consequences

**Immediate:**

- Implementation plan is: (a) delete Finding-4 artifacts, (b) add handle dir creation in `csq run`, (c) rewrite `csq swap` as symlink repoint, (d) simplify fanout, (e) update the rule and archive docs. Each step is scoped in `specs/02-csq-handle-dir-model.md`.
- Existing `config-N` dirs do NOT need migration. They become the permanent canonical homes under the new model with no schema change. The only change on upgrade is that new `csq run` creates `term-<pid>` instead of reusing `config-<N>` as the live dir.
- Running legacy-mode terminals (already launched with `CLAUDE_CONFIG_DIR=config-<N>` before the upgrade) keep working. `csq swap` inside them surfaces a clear error telling the user to relaunch with `csq run N` to get per-terminal behavior.

**Longer term:**

- The "stale session" rule (rule 7) disappears. The "slot vs account" identity derivation rule (rule 5) simplifies — there is no slot, only permanent accounts and ephemeral handles.
- Dashboard session list no longer needs a `needs_restart` badge or any restart-related UI.
- The tray quick-swap heuristic (journal 0018: "target the most recently modified credentials file") needs to be reconceived. In the handle dir model, the tray menu lists accounts, not config dirs, and a click targets a specific running terminal — probably the most recently active one or the user's explicit choice. Scoped to a follow-up; the retraction of the current mtime heuristic is the scope of this entry.

## For Discussion

1. Finding 4 was codified from one live debugging session where swap appeared to leave terminals stuck on old tokens. With hindsight, what prevented us from reading CC's source during that session before drafting the rule? Should codification steps include a CC-source verification checklist when the finding concerns CC behavior?
2. If `invalidateOAuthCacheIfDiskChanged` hadn't existed in CC (counterfactual — suppose Anthropic never fixed CC-1096), what would the correct csq architecture have looked like? Would any design recover swap-is-in-flight without requiring CC changes we don't control?
3. The handle-dir model makes `broker::fanout` essentially trivial and `rotation::swap_to`'s credential-write path disappear entirely. That's two complex subsystems collapsing at once. How much of csq's current complexity comes from working around Finding 4's incorrect model, and how much is inherent to the multi-account problem? A focused audit of the delta would be useful for estimating the cleanup cost.
