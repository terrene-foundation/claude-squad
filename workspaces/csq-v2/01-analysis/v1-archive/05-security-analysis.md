# Security Analysis: csq v2.0 Rust+Tauri Rewrite

Security review of the current Python codebase and analysis of security requirements for the Rust+Tauri rewrite.

**Reviewer**: security-reviewer agent
**Date**: 2026-04-10
**Files reviewed**: rotation-engine.py, dashboard/refresher.py, dashboard/oauth.py, dashboard/accounts.py, csq, statusline-quota.sh, install.sh, .gitignore, rules/security.md

---

## 1. Current Security Model (Python)

### 1.1 Credential Storage

**File locations** (all under `~/.claude/accounts/`):

| File                         | Purpose                                            | Permissions         |
| ---------------------------- | -------------------------------------------------- | ------------------- |
| `credentials/N.json`         | Canonical OAuth credentials per account            | 0o600               |
| `config-N/.credentials.json` | Live credentials for running CC instance           | 0o600               |
| `config-N/.csq-account`      | Account identity marker (durable across refreshes) | 0o600               |
| `config-N/.current-account`  | Statusline display account                         | 0o600               |
| `config-N/.live-pid`         | CC process ID for snapshot gating                  | best-effort         |
| `config-N/.quota-cursor`     | Stale-payload deduplication                        | best-effort         |
| `quota.json`                 | Per-account quota from statusline                  | 0o600 via `_save()` |
| `profiles.json`              | Email-to-account mapping                           | standard            |

**macOS Keychain**: Per-config-dir keychain entries use the service name `Claude Code-credentials-{sha256(config_dir)[:8]}`. The hash is derived from the NFC-normalized config dir path via `_keychain_service()`. Keychain is best-effort only; `.credentials.json` is the primary credential source.

**Directory permissions**: `install.sh` sets `chmod 700` on `~/.claude/accounts/` and `~/.claude/accounts/credentials/` (line 90). No-op on Windows.

### 1.2 Token Refresh and Monotonicity Guard

Three independent token refresh paths exist:

1. **`refresh_token()` in rotation-engine.py** (line 695): Reads canonical `credentials/N.json`, POSTs to Anthropic, writes new tokens atomically. No monotonicity guard at this level -- callers are expected to hold a lock.

2. **`broker_check()` in rotation-engine.py** (line 1702): Single-refresher-per-account pattern. Uses `_try_lock_file()` on `credentials/N.refresh-lock` for non-blocking lock. Re-reads canonical inside the lock. Fans out to all `config-X/.credentials.json` where marker matches. Recovery path (`_broker_recover_from_live`) promotes live sibling tokens when canonical RT is dead.

3. **`TokenRefresher._do_refresh()` in dashboard/refresher.py** (line 251): Has an explicit monotonicity guard: reads expiresAt before HTTP refresh, re-reads after, and skips the write if `post_expires_at > pre_expires_at` (line 314). This prevents concurrent dashboards from ping-ponging.

4. **`backsync()` in rotation-engine.py** (line 1366): Bidirectional sync with monotonicity: only overwrites canonical when `live_expires > canon_expires` (line 1427). Re-reads inside the lock and aborts if `cur_expires >= live_expires` (line 1490).

5. **`pullsync()` in rotation-engine.py** (line 1814): Canonical-to-live sync with strict-newer check: `canon_expires > live_expires` (line 1864).

**Assessment**: The monotonicity model is well-designed. The broker + backsync + pullsync triangle converges on "newest token wins everywhere." The re-read-inside-lock pattern in backsync() is particularly robust.

### 1.3 Atomic Write Patterns

The codebase uses two atomic write patterns:

**Pattern A -- `_save()` / `_atomic_replace()`** (rotation-engine.py, lines 243-249):

```
tmp = path.with_suffix(".tmp")
tmp.write_text(json.dumps(data))
_secure_file(tmp)          # chmod 0o600
_atomic_replace(tmp, path) # os.replace with Windows retry
```

**Pattern B -- inline temp+replace** (dashboard/refresher.py, dashboard/oauth.py):

```
tmp_file = cred_file.with_suffix(".tmp")
tmp_file.write_text(json.dumps(data, indent=2))
os.chmod(str(tmp_file), 0o600)
os.replace(str(tmp_file), str(cred_file))
os.chmod(str(cred_file), 0o600)  # belt and suspenders
```

Both patterns use `os.replace()` which is atomic on POSIX and near-atomic on Windows (NTFS `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`).

### 1.4 Crash Safety

**Well-protected**:

- All credential writes in `rotation-engine.py` use `_save()` or inline temp+replace
- `backsync()` acquires a per-canonical lock before writing, re-reads inside the lock
- `broker_check()` uses try-lock so only one terminal refreshes per cycle
- `_broker_recover_from_live()` restores original dead canonical on total failure (line 1687)

**Not atomic** (findings below):

- `csq cmd_login` inline Python writes canonical and live credentials with `open(cred_file, 'w')` (csq lines 156-164)
- `snapshot_account()` writes `.current-account` and `.live-pid` directly (rotation-engine.py lines 640-641)
- `which_account()` writes `.current-account` directly (rotation-engine.py line 310)

### 1.5 Keychain Integration

macOS keychain writes use `subprocess.run()` with array form and a 3-second timeout (rotation-engine.py line 810):

```python
subprocess.run(["security", "add-generic-password", ...], timeout=3)
```

The keychain service name is derived from `_keychain_service()` (line 774) using SHA-256 hash of the NFC-normalized config dir path. Never user-supplied.

Keychain reads in `csq cmd_login` also use array-form subprocess with timeout (csq line 127).

### 1.6 Secret Leak Prevention

**Token logging policy**: Access tokens are logged as prefix-only:

- `refresher.py` line 363: `access_token[:8]`
- `oauth.py` line 197: `access_token[:8]`

**Diagnostic output in swap_to()**: Refresh token prefixes (first 20 chars) appear in DIAG messages (rotation-engine.py lines 976, 1001, 1005). These are intentional diagnostics for debugging stuck-swap scenarios. 20-character prefixes of refresh tokens are not exploitable on their own but are more exposure than needed.

**No full tokens in logs**: Verified. No `print(f"token: {access_token}")` patterns exist in production code.

### 1.7 Input Validation

`_validate_account()` (line 684): Validates digits-only, range 1..999. Called at every CLI entry point (swap, init-keychain, email). The `csq` bash script also validates with `[[ "$n" =~ ^[1-9][0-9]*$ ]]` (line 88).

`csq_account_marker()` (line 552): Validates the marker content: `n.isdigit() and 1 <= int(n) <= MAX_ACCOUNTS` (line 577).

**Assessment**: Input validation is thorough on CLI paths.

---

## 2. Current Codebase Findings

### CRITICAL (Must fix before commit)

**C1. Non-atomic credential writes in `csq cmd_login`**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/csq`, lines 155-166

The inline Python in `cmd_login` writes both canonical and live credentials using `open(cred_file, 'w')` + `json.dump()` -- not temp+replace:

```python
with open(cred_file, 'w') as f:
    json.dump(creds, f, indent=2)
```

If the process is killed mid-write (e.g., user ctrl-C during login), the canonical `credentials/N.json` is left half-written. Any subsequent `csq swap N` from another terminal reads the corrupt file and writes corrupt tokens to `.credentials.json`, cascading the failure.

**Fix**: Use temp file + `os.replace()` pattern matching the rest of the codebase.

### HIGH (Should fix before merge)

**H1. `.gitignore` missing `config-*/` and `.credentials.json`**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/.gitignore`

The security rules (rules/security.md, line 68-71) require `.gitignore` to list:

- `.env` -- present
- `credentials/` -- present
- `config-*/` -- MISSING
- `.credentials.json` -- MISSING

While `config-*/` directories live under `~/.claude/accounts/` (not in the repo), a developer who accidentally creates a `config-test/` directory or a `.credentials.json` in the repo root during debugging would have no gitignore protection.

**Fix**: Add `config-*/` and `.credentials.json` to `.gitignore`.

**H2. Non-atomic writes for `.current-account` and `.live-pid` in `snapshot_account()`**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/rotation-engine.py`, lines 640-641

```python
(Path(config_dir) / ".current-account").write_text(account)
pid_file.write_text(str(cc_pid))
```

These are direct writes without temp+replace. `.current-account` is not a credential file, but a crash between the two writes leaves `.current-account` updated while `.live-pid` is stale, which could cause the snapshot to skip re-identification on the next CC restart.

**Risk**: Low data-loss risk (no credentials involved), but inconsistent state can cause the statusline to show the wrong account until the next CC restart. The atomic write pattern is already available via `_save()`.

**H3. Refresh token prefix exposed at 20 characters in DIAG messages**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/rotation-engine.py`, lines 976, 1001, 1005

The swap verification diagnostics log refresh token prefixes at 20 characters:

```python
rb_rt = readback.get("claudeAiOauth", {}).get("refreshToken", "")[:20]
```

The security rules specify 8-character prefix + 4-character suffix for diagnostics. 20 characters of a refresh token is more exposure than necessary for swap debugging.

**Fix**: Reduce to `[:8]...[-4:]` pattern consistent with the rest of the codebase.

**H4. `profiles.json` write in `csq cmd_login` is not atomic**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/csq`, lines 186-191

```python
with open(f, 'w') as fh:
    json.dump(d, fh, indent=2)
```

While `profiles.json` does not contain tokens (only emails), a crash mid-write corrupts the email-to-account mapping, breaking `csq status` and `csq suggest` for all terminals.

### MEDIUM (Fix in next iteration)

**M1. `dashboard/accounts.py` does not `_secure_file()` on temp file before replace**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/dashboard/accounts.py`, lines 292-299

The temp file is written and replaced, but `os.chmod` is called only on the final file, not the temp. Between `write_text()` and `os.replace()`, the temp file has default permissions (likely 0o644 on most systems). On a multi-user system, another process could read the temp file in that window.

**M2. `install.sh` settings.json write is not atomic**

File: `/Users/esperie/repos/terrene/contrib/claude-squad/install.sh`, line 181

```python
with open(f,'w') as fh: json.dump(s, fh, indent=2)
```

`settings.json` does not contain credentials, but a corrupt settings file on a crashed install requires manual recovery.

**M3. `which_account()` writes `.current-account` directly (line 310)**

Non-atomic write, same pattern as H2 but in a different code path. Low impact because it's only the initial-state fallback.

### LOW (Consider fixing)

**L1. CLIENT_ID duplicated across three files**

The OAuth CLIENT_ID (`9d1c250a-e61b-44d9-88ed-5944d1962f5e`) is defined independently in `rotation-engine.py`, `dashboard/refresher.py`, and `dashboard/oauth.py`. If Anthropic changes the client ID, all three must be updated. In the Rust rewrite, define it once as a constant.

**L2. User-Agent string hardcoded to `claude-code/2.1.91`**

All three OAuth callers hardcode the User-Agent. This is not a security vulnerability but could cause Anthropic to reject requests if they enforce User-Agent versioning.

### PASSED CHECKS

- **Secrets detection**: No hardcoded API keys or real tokens in source. The `sk-ant-` patterns in bench-results files are synthetic test prompts (e.g., `sk-ant-api03-abc123`), not real credentials.
- **No `shell=True`**: All `subprocess.run()` calls use array form. No shell injection vectors.
- **No secrets in logs**: Full token values never appear in print/log statements.
- **Input validation on account numbers**: `_validate_account()` called at every CLI entry point. `csq` bash validation with regex on entry.
- **Keychain service name**: Always derived from `_keychain_service()`, never from user input.
- **File permissions**: `_secure_file()` called on all credential writes in `rotation-engine.py`. Dashboard files also chmod 0o600.
- **Fail-closed on keychain**: 3-second timeout on keychain writes, failures non-fatal.
- **Fail-closed on lock contention**: `_try_lock_file()` returns None on failure, callers skip and retry.
- **Concurrency monotonicity**: backsync, pullsync, and broker all check `expiresAt` monotonicity before writing.
- **`.env` in .gitignore**: Present.
- **`credentials/` in .gitignore**: Present.

---

## 3. Rust Migration Security Considerations

### 3.1 Patterns That Translate Directly

| Python Pattern                            | Rust Equivalent                                                | Notes                                                             |
| ----------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------- |
| `_atomic_replace()` (temp + `os.replace`) | `tempfile::NamedTempFile` + `persist()` or `std::fs::rename()` | `tempfile` crate handles cleanup on drop                          |
| `_secure_file()` (chmod 0o600)            | `std::os::unix::fs::PermissionsExt::set_permissions()`         | Unix-only; Windows needs `windows-acl` crate or no-op             |
| `_validate_account()`                     | Newtype pattern: `struct AccountNum(u16)` with `TryFrom`       | Compiler enforces validation at construction                      |
| `json.loads/dumps`                        | `serde_json`                                                   | Struct-typed deserialization catches schema drift at compile time |
| `fcntl.flock()`                           | `fs2::FileExt::lock_exclusive()` / `try_lock_exclusive()`      | Cross-platform file locking                                       |
| `ctypes.windll.kernel32` (Windows mutex)  | `windows-sys` crate for named mutexes                          | Explicit handle types eliminate the truncation bug risk           |
| `hashlib.sha256` (keychain service hash)  | `sha2` crate                                                   | Identical algorithm                                               |

### 3.2 Patterns That Need New Approaches in Rust

**Secret types**: Python relies on discipline to avoid logging tokens. Rust can enforce it structurally:

```rust
/// Access token that cannot be accidentally logged.
/// Display shows only the prefix; Debug is redacted.
pub struct AccessToken(String);

impl std::fmt::Display for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 12 {
            write!(f, "{}...{}", &self.0[..8], &self.0[self.0.len()-4..])
        } else {
            write!(f, "[redacted]")
        }
    }
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccessToken([redacted])")
    }
}
```

This makes `format!("{}", token)` safe by construction. The inner value is accessible only via an explicit `.expose()` method. Apply the same pattern to `RefreshToken`.

**Account number validation**: Replace runtime validation with a newtype:

```rust
pub struct AccountNum(u16);

impl TryFrom<u16> for AccountNum {
    type Error = AccountError;
    fn try_from(n: u16) -> Result<Self, Self::Error> {
        if n >= 1 && n <= MAX_ACCOUNTS {
            Ok(AccountNum(n))
        } else {
            Err(AccountError::InvalidAccountNum(n))
        }
    }
}
```

This eliminates the possibility of an unvalidated account number reaching a file path. The type system enforces what `_validate_account()` enforces at runtime.

**Atomic file writes**: The `tempfile` crate provides `NamedTempFile` with `persist()`, which does `rename` under the hood:

```rust
use tempfile::NamedTempFile;

fn atomic_write(target: &Path, data: &[u8]) -> io::Result<()> {
    let dir = target.parent().ok_or(io::ErrorKind::InvalidInput)?;
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(data)?;
    // Set permissions before persisting
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file().set_permissions(
            std::fs::Permissions::from_mode(0o600)
        )?;
    }
    tmp.persist(target)?;
    Ok(())
}
```

The advantage over the Python pattern: if the process crashes before `persist()`, the `Drop` impl on `NamedTempFile` automatically cleans up the temp file. In the Python code, a crash between `write_text()` and `os.replace()` leaves a `.tmp` file on disk.

### 3.3 Recommended Rust Crates

| Concern                    | Crate                         | Notes                                         |
| -------------------------- | ----------------------------- | --------------------------------------------- |
| Keychain (macOS)           | `security-framework`          | Native Keychain Services API via FFI. Mature. |
| Secret Service (Linux)     | `secret-service` or `keyring` | D-Bus interface to GNOME Keyring / KWallet    |
| Windows Credential Manager | `keyring` (cross-platform)    | `keyring` crate wraps all three platforms     |
| Atomic file writes         | `tempfile`                    | `NamedTempFile::persist()` for atomic rename  |
| File locking               | `fs2`                         | Cross-platform `flock` / `LockFileEx`         |
| JSON serialization         | `serde_json` + `serde`        | Typed (de)serialization                       |
| HTTP client                | `reqwest` (with rustls)       | TLS via rustls avoids OpenSSL supply chain    |
| SHA-256                    | `sha2`                        | Pure Rust, audited                            |
| PKCE                       | `rand` + `sha2` + `base64`    | Standard library equivalent                   |
| Secrets in memory          | `secrecy`                     | `Secret<T>` with zeroize-on-drop              |
| Crate auditing             | `cargo-audit`                 | Check for known vulnerabilities               |

**Important**: The `secrecy` crate provides `Secret<String>` which implements `Zeroize` on drop and prevents accidental Display/Debug. This is the Rust-idiomatic way to handle tokens in memory and directly addresses the "no tokens in logs" requirement.

### 3.4 Tauri-Specific Security

**IPC Allowlist**: Tauri v2 uses a capability-based permission system. The webview can only invoke commands explicitly listed in `capabilities/*.json`. For csq:

```json
{
  "identifier": "main",
  "windows": ["main"],
  "permissions": [
    "csq:allow-list-accounts",
    "csq:allow-get-status",
    "csq:allow-swap-account",
    "csq:allow-refresh-token",
    "csq:allow-get-quota"
  ]
}
```

Commands that MUST NOT be exposed to the webview:

- Raw file system access (no `fs:allow-read`, `fs:allow-write`)
- Shell execution (no `shell:allow-execute`)
- Direct credential file reads (the Tauri command returns sanitized data, not raw JSON)

**CSP Headers**: The Tauri webview MUST use a strict Content Security Policy:

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
connect-src https://platform.claude.com;
img-src 'self' data:;
```

- No `connect-src *` -- only allow connections to the Anthropic token endpoint
- No `script-src 'unsafe-eval'` -- prevents XSS escalation
- No `connect-src http:` -- all external connections must be HTTPS

**Webview Isolation**: Tauri v2's isolation pattern runs IPC through a separate iframe with a cryptographic key. Enable this:

```json
{
  "security": {
    "csp": "...",
    "dangerousDisableAssetCspModification": false,
    "freezePrototype": true
  }
}
```

`freezePrototype: true` prevents prototype pollution attacks in the webview.

---

## 4. Threat Model for Desktop App

### 4.1 Local Attacker (Malware on Same Machine)

**Attack surface**: Credential files on disk, process memory, IPC channel.

**Mitigations**:

- Credential files at 0o600 (POSIX) -- readable only by the owning user
- Directory at 0o700 -- not listable by other users
- macOS Keychain provides hardware-backed storage with user authentication
- `secrecy` crate zeroizes tokens in memory on drop
- Tauri IPC is local-only (no network-accessible endpoint)

**Residual risk**: A process running as the same user can read any file the user owns. This is inherent to the threat model -- csq cannot defend against malware running as the same user. The keychain is the strongest defense here because it requires user authentication (biometric/password) on macOS.

### 4.2 Network Attacker

**Attack surface**: Token refresh HTTPS calls, OAuth callback on localhost.

**Mitigations**:

- Use `reqwest` with `rustls` (pure-Rust TLS) -- no OpenSSL dependency
- OAuth callback listener binds to `127.0.0.1` only (not `0.0.0.0`)
- PKCE prevents authorization code interception
- State parameter prevents CSRF on the callback

**Consideration for v2.0**: Certificate pinning for `platform.claude.com` is possible via `reqwest`'s `danger_accept_invalid_certs(false)` (default) plus a custom certificate verifier. However, pinning adds operational fragility (Anthropic certificate rotation breaks pinned clients). Recommend: rely on system CA store validation, do not pin.

### 4.3 XSS in Tauri Webview

**Attack surface**: If an attacker injects script into the webview, they could invoke IPC commands.

**Mitigations**:

- Strict CSP (no `unsafe-eval`, no external script sources)
- Tauri IPC allowlist limits available commands
- Tauri isolation pattern adds cryptographic verification to IPC calls
- `freezePrototype: true` prevents prototype pollution
- The webview renders local content only (no remote pages)
- Tauri commands validate all arguments server-side (Rust)

**Residual risk**: If an XSS vector exists in the local webview content and the CSP is misconfigured, the attacker can invoke any allowed IPC command. This means they could trigger a swap or read account status, but cannot exfiltrate raw tokens if the Tauri commands return sanitized data.

### 4.4 Supply Chain

**Attack surface**: Compromised Rust crate injects token-stealing code.

**Mitigations**:

- `cargo-audit` in CI to check for known vulnerabilities
- `cargo-vet` to track crate audit status
- Pin exact crate versions in `Cargo.lock` (committed to git)
- Minimize dependency count -- csq's core logic (file I/O, JSON, HTTP) needs few crates
- Use `rustls` instead of `openssl-sys` to avoid native dependency supply chain

**Recommended crate audit policy**: Every crate that handles tokens (keyring, reqwest, serde_json) must be audited before adoption. Use `cargo-crev` or `cargo-vet` to record audit decisions.

### 4.5 Multi-Process Race -- Single-Use Refresh Token Contention

**Attack surface**: Anthropic rotates refresh tokens on each use. If two processes refresh simultaneously, one gets a new RT and the other's RT is invalidated.

**Current mitigation** (Python): The broker pattern (`broker_check()`) with per-account try-lock ensures only one terminal refreshes at a time. Recovery path promotes live sibling tokens. This is the most sophisticated part of the current security model.

**Rust migration**: The broker pattern MUST be preserved. Use `fs2::FileExt::try_lock_exclusive()` for the per-account refresh lock. The fan-out pattern (write to all config dirs with matching marker) must also be preserved.

**New consideration for Tauri**: The Tauri app is a single process (unlike 15 separate csq terminals). Token refresh can be centralized in a single async task with `tokio::sync::Mutex`, eliminating the file-lock-based broker entirely for the desktop app. However, the file-lock broker must still exist for CLI compatibility (csq terminals running alongside the Tauri app).

---

## 5. Security Requirements for v2.0

### MUST (Blocking)

| ID  | Requirement                                                                                    | Rationale                                       |
| --- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| S1  | Credential file permissions 0o600 on POSIX, no-op on Windows                                   | Multi-user machine protection                   |
| S2  | Atomic writes for all credential mutations (temp + rename)                                     | Crash safety for 15+ concurrent terminals       |
| S3  | Monotonicity guard for refresh token rotation (expiresAt strictly newer)                       | Prevents ping-pong between concurrent terminals |
| S4  | No tokens in logs -- `Display` trait on token types shows prefix only                          | Tokens in bug reports = credential leak         |
| S5  | Tauri IPC allowlist -- only expose account-management commands                                 | Minimizes webview attack surface                |
| S6  | CSP headers -- `default-src 'self'`, no `unsafe-eval`, connect only to `platform.claude.com`   | Prevents XSS escalation                         |
| S7  | Account number validation via newtype (`AccountNum`)                                           | Path traversal prevention at compile time       |
| S8  | No `shell=True` equivalent -- all process spawning via `std::process::Command` with array args | Shell injection prevention                      |
| S9  | `.env`, `credentials/`, `config-*/`, `.credentials.json` in `.gitignore`                       | Prevent accidental credential commits           |
| S10 | `secrecy::Secret<String>` for token storage in memory with zeroize-on-drop                     | Minimizes memory exposure window                |
| S11 | Broker pattern preserved -- per-account lock for refresh, fan-out to all live configs          | Prevents refresh token contention               |
| S12 | OAuth callback listener binds to 127.0.0.1 only                                                | Prevents network interception of auth codes     |
| S13 | PKCE (S256) for all OAuth flows                                                                | Prevents authorization code interception        |

### SHOULD (High priority)

| ID  | Requirement                                                | Rationale                          |
| --- | ---------------------------------------------------------- | ---------------------------------- |
| S14 | macOS Keychain integration via `security-framework` crate  | Hardware-backed credential storage |
| S15 | Linux Secret Service integration via `keyring` crate       | Desktop keyring storage            |
| S16 | Windows Credential Manager integration via `keyring` crate | Platform-native credential storage |
| S17 | `cargo-audit` in CI pipeline                               | Automated vulnerability detection  |
| S18 | Tauri isolation pattern enabled                            | Cryptographic IPC verification     |
| S19 | `freezePrototype: true` in Tauri config                    | Prototype pollution defense        |
| S20 | TLS via `rustls` (not `openssl-sys`)                       | Avoids native OpenSSL supply chain |
| S21 | `cargo-vet` or `cargo-crev` for crate audit tracking       | Supply chain governance            |

### MAY (Nice to have)

| ID  | Requirement                                        | Rationale                               |
| --- | -------------------------------------------------- | --------------------------------------- |
| S22 | Certificate pinning for `platform.claude.com`      | Defense against CA compromise (fragile) |
| S23 | Token encryption at rest (beyond file permissions) | Defense-in-depth for disk-based storage |
| S24 | Process memory locking (`mlock`) for token pages   | Prevents swapping tokens to disk        |

---

## 6. Migration Checklist

For each security-sensitive component, track migration status:

- [ ] `_atomic_replace()` -> `tempfile::NamedTempFile::persist()`
- [ ] `_secure_file()` -> `std::os::unix::fs::PermissionsExt`
- [ ] `_validate_account()` -> `AccountNum` newtype with `TryFrom`
- [ ] `_keychain_service()` -> `sha2` hash, `security-framework` write
- [ ] `_lock_file()` / `_try_lock_file()` -> `fs2::FileExt`
- [ ] `refresh_token()` -> `reqwest` POST with `Secret<String>`
- [ ] `broker_check()` -> async task with file lock
- [ ] `backsync()` / `pullsync()` -> preserve monotonicity guards
- [ ] Token logging -> `Display` trait redaction on `AccessToken`, `RefreshToken`
- [ ] PKCE generation -> `rand::OsRng` + `sha2` + `base64`
- [ ] Tauri IPC -> capability allowlist
- [ ] Tauri CSP -> strict policy in `tauri.conf.json`
- [ ] `.gitignore` -> include all credential paths
- [ ] CI -> `cargo-audit`, `cargo-vet`
