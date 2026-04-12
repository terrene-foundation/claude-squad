# 06 Keychain Integration

Spec version: 1.0.0 | Status: DRAFT | Governs: macOS Keychain writes, service-name derivation parity, Linux/Windows fallback

---

## 6.0 Scope

This spec defines csq's interaction with platform credential stores. It is a consumer of spec 01 sections 1.3, 1.5, and 1.7, which describe CC's own behavior. csq's job is to remain parity-compatible with CC so that CC's keychain-fallback read path finds what csq wrote.

## 6.1 macOS service-name derivation (MUST match CC)

**Spec 01 section 1.3** defines the formula CC uses:

```
Default dir (no CLAUDE_CONFIG_DIR):  "Claude Code-credentials"
Custom dir:                          "Claude Code-credentials-<sha256(dir)[:8]>"
```

The hash is computed over the RAW config dir path (NFC-normalized in macOS filesystem terms). csq MUST reproduce this formula exactly. Any divergence causes CC's keychain-fallback read to miss entries csq wrote, surfacing as "Not logged in" for accounts that are actually provisioned.

**Parity test:** csq's unit test suite has a golden-values test (journal 0003 Finding 5) that hashes a fixed set of paths and compares to values computed from the CC source. Any CC update that changes the formula breaks this test and is caught before the csq build ships.

## 6.2 Keychain write path (macOS)

csq writes to the Keychain via `security add-generic-password` shelled-out with a hex payload passed over stdin. This matches what CC does at `src/utils/secureStorage/macOsKeychainStorage.ts:97-157`. See spec 01 section 1.7 for the exact CC flow.

**csq's writes are per-config-dir**:

- For each account N, csq writes to service `Claude Code-credentials-<sha256(config-<N> path)[:8]>`.
- For each handle dir `term-<pid>`, csq does NOT proactively write. The handle dir's `.credentials.json` symlink resolves to `config-<N>/.credentials.json` which CC reads directly; CC's keychain lookup uses service name `Claude Code-credentials-<sha256(term-<pid> path)[:8]>` which MAY not exist. When that keychain entry is missing, CC falls back to reading the symlinked file — which is exactly what we want.
- **Consequence:** handle dirs have NO keychain entries in the normal case. The per-handle hash namespace is a fallback that CC never needs to hit.

**Why not write per-handle?** Each handle dir lives for the life of one `claude` process, often minutes to hours. Writing a keychain entry per handle dir would accumulate hundreds of stale entries over time, all of which CC would read and cache at startup. The file-through-symlink path is faster, cleaner, and has no cleanup cost (symlinks disappear with the handle dir on sweep).

## 6.3 CC's 30-second cache and the csq swap latency bound

Spec 01 section 1.5 documents CC's per-process 30-second keychain read cache. This matters for csq swap's latency:

- When csq swap repoints the handle dir's `.credentials.json` symlink to a new account's file, CC's next `fs.stat` on `.credentials.json` follows the symlink and sees the new mtime. Cache is cleared, credentials re-read. **The read path goes through `secureStorage.read()` which tries the keychain FIRST**.
- The keychain has its own 30-second per-process TTL. If this CC process read the keychain within the last 30 seconds (for whatever reason), it serves that cached value — which is for the OLD account, not the new one.
- **Result:** swap latency is effectively instant IF the csq swap writes the fresh credentials via the file path (symlink resolves → new file → mtime check → memoize cleared → next read hits plaintext fallback). If it writes only via keychain, the current terminal may see up to 30 seconds of stale reads.

**This is why csq swap is a symlink-repoint operation, not a keychain-write operation.** Repointing the symlink makes the change visible instantly through the file path. See spec 02 section 2.3.3 INV-04 for the normative statement.

## 6.4 Write-path guards

When csq writes to `config-<N>/.credentials.json` (via daemon refresher or `csq login N`):

1. **Atomic**: temp file + rename, owner permissions `0o600`.
2. **Subscription metadata preserved**: if the new tokens have `null` for `subscription_type` or `rate_limit_tier`, read the existing file and preserve non-null values. See spec 01 section 1.7 and `rules/account-terminal-separation.md` rule 4.
3. **Keychain mirror**: after successful file write, also write to the keychain entry for `config-<N>`'s path. Best-effort; a keychain failure does NOT fail the operation because the file write is authoritative.
4. **Secure file permissions**: `platform::fs::secure_file()` sets `0o600`. Windows is a no-op.

## 6.5 Linux and Windows

**Linux:** no keychain integration is required for the normal CC flow. CC on Linux uses `plainTextStorage` directly (see `src/utils/secureStorage/index.ts:11`). csq writes to the file with `0o600` and stops. A future `libsecret` integration is tracked but not in current scope.

**Windows:** named pipe IPC (M8.6) is pending; for credentials, plaintext file with ACLs via `secure_file()` is the current path. Windows Credential Manager integration is tracked but not in current scope.

## 6.6 Keychain cleanup on account deletion

When an account is deleted (via a `csq delete N` command, pending — not currently implemented):

1. Remove `config-<N>/`.
2. Delete the keychain entry via `security delete-generic-password -s "Claude Code-credentials-<hash>"`.
3. Remove `credentials/<N>.json`.
4. Remove profile entry in `profiles.json`.
5. Signal the daemon to stop refresh and polling for N.

## 6.7 Cross-references

- `specs/01-cc-credential-architecture.md` sections 1.3, 1.5, 1.7 — CC's own keychain behavior (authoritative).
- `specs/02-csq-handle-dir-model.md` section 2.3.3 — why swap is a symlink repoint, not a keychain write.
- `rules/security.md` — credential handling invariants, atomic writes, token redaction.
- `rules/account-terminal-separation.md` rule 4 — subscription-metadata preservation.
- Journal `0003-RISK-redteam-m1-m2-findings.md` — keychain service-name parity test.
- Journal `0013-DISCOVERY-macos-security-no-stdin-password.md` — why csq shells out to `security` instead of using native FFI.
- Journal `0028-DECISION-account-terminal-separation-python-elimination.md` — GUI-app keychain peculiarities (e.g. Tauri missing `$USER`).

## Revisions

- 2026-04-12 — 1.0.0 — Initial draft. Per-handle-dir keychain writes explicitly rejected in favor of symlink-through-file reads.
