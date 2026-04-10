# Red Team Report: v2.0 Analysis Specs

## Date: 2026-04-10

## Agent: deep-analyst

## Scope: All analysis docs, plans, user flows — cross-referenced against v1.x source

---

## CRITICAL (3)

### C1. Swap doesn't work — CC caches credentials in memory

journal/0014 documents that `csq swap` writes correct credentials to disk, but the running CC instance does NOT re-read `.credentials.json` until its cached token expires or receives a 401. The user flows (02-daily-use, 03-rate-limit-recovery) and the daemon-based auto-rotation all assume swap is picked up "on the next API call." This is the core user-facing promise of the product, and it's documented as false in the journal.

**Impact**: Auto-rotation is non-functional if CC doesn't re-read credentials. The entire daemon value proposition collapses.

**Resolution needed**: Either (a) document CC's actual reload behavior and design around it (force 401 by revoking old token?), (b) implement a workaround from journal/0014, or (c) file upstream feature request for `claude auth reload`.

### C2. `dashboard-accounts.json` missing from migration spec

NFR file layout (section 4.3) omits this file, yet `dashboard/accounts.py` reads/writes it and functional requirements reference it. Manually-added accounts from v1.x dashboard use will be silently lost on migration.

### C3. `keyring` crate stores plain UTF-8 but CC expects hex-encoded JSON

ADR-003 says "store JSON directly" via keyring crate. But v1.x writes hex-encoded JSON via `security add-generic-password -X`, and CC's reader hex-decodes the result. keyring crate stores plain UTF-8 strings. During coexistence, CC reads v2.0's keychain entry, tries to hex-decode a JSON string, gets garbage, falls through to `.credentials.json`. Works until the next rotation cycle creates a mismatch.

**Resolution needed**: Use `security-framework` crate directly with hex-encoding, or accept/document the incompatibility.

---

## HIGH (6)

### H1. Auto-rotation has no spec

User flows promise `csq config set auto-rotate true` — this command doesn't exist anywhere. No functional requirement, no scope matrix entry, no implementation plan coverage.

### H2. Credential refresh silently drops `subscriptionType` and `rateLimitTier` fields

Migration says "same format, no conversion" — but Rust serde struct must include these optional fields and carry them through refresh. If defined strictly from OAuth response schema, they're silently dropped on first refresh.

### H3. `check` command missing from scope matrix

`rotation-engine.py` implements it, functional requirements list it (CL-021), scope matrix omits it.

### H4. ADR says "Svelte 4/5" but project rules mandate Svelte 5 runes

ADR-002 mentions `$:` reactive declarations (Svelte 4, deprecated). `svelte-patterns.md` mandates `$state/$derived/$effect` and lists `$:` as anti-pattern.

### H5. Windows `WAIT_ABANDONED` not handled in locking spec

POSIX flock releases on crash. Windows named mutex returns `WAIT_ABANDONED` which v1.x treats as failure = permanently held lock until reboot. v2.0 must handle this. Documented in journal/0013 but not in analysis.

### H6. Broker exit code 2 signaling has no v2.0 equivalent

v1.x uses subprocess exit codes. v2.0 daemon uses in-process function calls returning `Result`. Error propagation model for "total failure" undefined.

---

## MEDIUM (7)

- M1: Three different polling intervals (5min Anthropic, 15min 3P, 5min token health) — implementation plan says "5 minutes" without distinguishing
- M2: `init_keychain` command not in scope matrix
- M3: `cleanup()` targets `.account.*` files not in file layout
- M4: User-Agent string `claude-code/2.1.91` hardcoded — needs decision for v2.0
- M5: Refresh window changed from 30min (dashboard) to 2hr (daemon) without flagging
- M6: `csq daemon start/stop/status` commands not in functional requirements
- M7: File locking crate (`fs2`) not specified — `fs2` uses `LockFileEx` on Windows (byte-range locks) vs v1.x `CreateMutexW` (named mutexes) — different semantics

---

## LOW (5)

- L1: OAuth CLIENT_ID and SCOPES duplicated in 3 files — need shared constants
- L2: `csq start` alias for `run` not documented
- L3: `csq ls` and `csq quota` aliases not in scope matrix
- L4: Phase 1 says "28 functions" but actually scopes 51 entries — effort estimate may be wrong
- L5: User flow shows `csq models switch` but v1.x uses positional args — breaking change

---

## Complexity Scores

| Component               | Score | Rating                                   |
| ----------------------- | ----- | ---------------------------------------- |
| Token Refresh Daemon    | 19    | Critical                                 |
| `csq run` Session Setup | 14    | High                                     |
| Keychain Service Name   | 9     | Medium (but mismatch impact is Critical) |
