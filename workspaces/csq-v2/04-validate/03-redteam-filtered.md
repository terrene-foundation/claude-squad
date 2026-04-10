# Red Team Report: v2.0 Analysis (Filtered)

## Date: 2026-04-10

## Agent: deep-analyst (round 2 — filtered for v2.0 relevance only)

---

## Previous Findings Reclassified

| ID     | Original | New        | Reason                                                                   |
| ------ | -------- | ---------- | ------------------------------------------------------------------------ |
| RT1-C1 | CRITICAL | **REJECT** | Swap works. journal/0016 supersedes 0014. CC has mtime-based reload.     |
| RT1-C2 | CRITICAL | **LOW**    | File listed in functional reqs + scope matrix. Missing from one diagram. |
| RT1-C3 | CRITICAL | **VALID**  | Hex encoding incompatibility — `keyring` crate vs CC keychain reader.    |
| RT1-H1 | HIGH     | **VALID**  | Auto-rotation promised in user flows, no spec.                           |
| RT1-H2 | HIGH     | **VALID**  | Credential field preservation needs explicit type definition.            |
| RT1-H3 | HIGH     | **LOW**    | Listed in functional reqs, missing from scope matrix only.               |
| RT1-H4 | HIGH     | **LOW**    | Wording inconsistency in one ADR sentence.                               |
| RT1-H5 | HIGH     | **VALID**  | Windows WAIT_ABANDONED handling unspecified.                             |
| RT1-H6 | HIGH     | **MEDIUM** | Standard Rust error handling, but error types undefined.                 |
| RT2-C1 | CRITICAL | **MEDIUM** | Socket path needs finalizing; same-user access is inherent.              |
| RT2-C2 | CRITICAL | **REJECT** | V1.x issue. V2.0 already mandates Ed25519 + checksum.                    |
| RT2-C3 | CRITICAL | **MEDIUM** | PKCE prevents code theft. Bind-check is impl detail.                     |
| RT2-H1 | HIGH     | **LOW**    | CSP/isolation already specified. Coding practice issue.                  |
| RT2-H2 | HIGH     | **VALID**  | OAuth state TTL genuinely missing.                                       |
| RT2-H3 | HIGH     | **REJECT** | V1.x issue. V2.0 daemon uses tokio::sync::Mutex per ADR-006.             |
| RT2-H4 | HIGH     | **REJECT** | V1.x bash issue. V2.0 reads keys from stdin in-process.                  |
| RT2-H5 | HIGH     | **LOW**    | Trivial .gitignore fix.                                                  |

**Result: 6 CRITICALs → 1 VALID. 11 HIGHs → 4 VALID.**

---

## 10 Gaps That Block Implementation — ALL RESOLVED

All gaps resolved in `01-analysis/01-research/07-gap-resolutions.md` (2026-04-10).

### GAP-1: Credential JSON schema — RESOLVED

Real file dumped. `CredentialFile` + `OAuthPayload` structs defined with `#[serde(flatten)]` for forward compat. Field preservation rule for refresh merging. `expiresAt` confirmed as milliseconds (not seconds).

### GAP-2: Keychain hex-encoding decision — RESOLVED

Decision: hex-encode on macOS via `security-framework` crate (CC compatibility). Direct JSON via `keyring` crate on Linux/Windows (CC doesn't read keychain there). ADR-003 amendment written.

### GAP-3: Quota JSON schema — RESOLVED

Real file dumped. `QuotaFile` + `AccountQuota` + `UsageWindow` structs defined. Key finding: `resets_at` is seconds (10-digit), `used_percentage` is float. Expiry logic specified.

### GAP-4: Error type hierarchy — RESOLVED

`CsqError` top-level enum with 7 module errors: `CredentialError`, `PlatformError`, `BrokerError`, `OAuthError`, `DaemonError`, `ConfigError`, plus `anyhow` catch-all. Tauri command mapping included.

### GAP-5: Cargo workspace structure — RESOLVED

Three-crate workspace: `csq-core` (library, zero Tauri dependency), `csq-cli` (binary, clap routing), `src-tauri/` (Tauri binary). Full `Cargo.toml` skeletons with workspace dependencies.

### GAP-6: Auto-rotation spec — RESOLVED

Trigger: 5h usage >= 95% + better account exists + terminal idle + 5-min cooldown. Config in `rotation.json`. Disabled by default. Per-terminal, not per-account. Daemon loop + CLI fallback specified.

### GAP-7: `profiles.json` schema — RESOLVED

`ProfilesFile` + `AccountProfile` structs defined with `#[serde(flatten)]`. Write, read, and 3P account patterns documented.

### GAP-8: Windows WAIT_ABANDONED handling — RESOLVED

Treat as "acquired with warning" (not failure). Log at warn level. Callers re-read and validate protected resources — existing broker/backsync patterns already do this. Platform locking spec amendment written.

### GAP-9: Daemon detection protocol — RESOLVED

PID file + socket paths per platform. 4-step liveness check: PID file → PID alive → socket connect (100ms) → health check (200ms). Timeouts, fallback behavior, startup sequence, and graceful shutdown all specified.

### GAP-10: OAuth state token TTL — RESOLVED

10-minute TTL on pending state tokens. Background cleanup every 60s. Bounded map (100 entries, evict oldest). Single-use guarantee (consume on callback).
