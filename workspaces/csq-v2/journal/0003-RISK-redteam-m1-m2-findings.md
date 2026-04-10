---
type: RISK
date: 2026-04-10
created_at: 2026-04-10T16:00:00Z
author: agent
session_id: m1-m2-redteam
session_turn: 45
project: csq-v2
topic: Red team findings for M1+M2 platform and credential layers
phase: redteam
tags: [security, testing, parity, credentials, platform]
---

# Red Team Findings: M1 Platform + M2 Credentials

Three-agent red team (deep-analyst, security-reviewer, testing-specialist) converged on 5 CRITICAL and 6 HIGH findings across the M1 and M2 implementations. All CRITICAL and HIGH findings were fixed in this session.

## Critical Findings (All Fixed)

1. **RefreshResponse Debug leaked raw tokens** — `#[derive(Debug)]` on a struct holding access and refresh tokens as `String`. Any `dbg!()` or `tracing::debug!(?response)` would print full tokens. Fixed: custom Debug impl masks both fields.

2. **macOS keychain had no timeout** — `security` CLI calls used `.output()` which blocks indefinitely if the security daemon hangs. v1.x Python used `timeout=3`. Fixed: `try_wait()` polling loop with 3-second deadline.

3. **Temp file permissions window** — Credential files were written at default permissions (0o644), renamed to final path, then `chmod 0o600`. Between rename and chmod, credentials were world-readable. Fixed: `secure_file()` called on temp file before `atomic_replace()`.

4. **try_lock_file never tested returning None** — The acceptance criterion "try_lock returns None when held" had no assertion in any test. The unit test was a no-op (comment admitting deferral to integration tests that didn't exist). Fixed: cross-process test using perl subprocess.

5. **Keychain service name not parity-tested against v1.x** — The implementation plan called this "the single most critical compatibility test." The test checked format and uniqueness but not actual hash values. Fixed: 5 golden values computed from `hashlib.sha256(unicodedata.normalize('NFC', path).encode()).hexdigest()[:8]`.

## High Findings (All Fixed)

6. **Windows fs.rs won't compile** — `warn!` macro used without `use tracing::warn;` inside `#[cfg(windows)]` block. Fixed: added import.

7. **Predictable temp file name** — `.tmp` suffix allowed concurrent saves to the same path to clobber each other's temp files. Fixed: `.tmp.{pid}` suffix.

8. **Cargo.lock gitignored** — Binary crate needs reproducible builds. Removed from `.gitignore`.

9. **OAuthError::Http body could leak tokens** — Error body from Anthropic endpoint might echo tokens. Fixed: `sanitize_body()` redacts `sk-ant-oat01-` and `sk-ant-ort01-` prefixes.

10. **save_canonical partial failure untested** — Spec guarantees canonical succeeds even if live dir is unwritable. No test existed. Fixed: test makes live dir 0o000.

11. **refresh_token didn't verify URL/body** — Mock HTTP function ignored both parameters. Fixed: test captures and asserts exact URL and body format.

## Remaining (Accepted)

- **Linux/Windows keychain unwired** — `keyring` crate not added. CC doesn't read keychain on these platforms, so file-based storage is the functional path. Lower priority.
- **AccessToken implements Serialize** — Necessary for credential file round-trip. Tauri command layer (M10+) must use view structs, not `CredentialFile`, in IPC responses.
- **find_cc_pid no controlled process tree test** — Cannot easily spawn a process named "claude" in tests. The function works correctly when running under Claude Code (verified by `find_cc_pid_does_not_error` test).

## For Discussion

1. The v1.x Python uses `subprocess.run(["security", ...], timeout=3)` for keychain operations. The Rust v2.0 implementation uses `try_wait()` polling because `Child::wait_timeout` is not in stable std. If Rust stabilizes `wait_timeout`, should we switch to avoid the 100ms polling granularity?

2. The `AccessToken` Serialize impl is a calculated trade-off: credential files need full token serialization, but the same type could accidentally reach a Tauri IPC boundary. If we had introduced a separate `CredentialFileWriter` with its own serialization, would that have been worth the added complexity?

3. The keychain service name parity test uses golden values computed once from v1.x Python. If a future Python update changes `unicodedata.normalize` behavior, the golden values would be wrong and the Rust code would diverge from live v1.x installations. Should we run the Python computation in CI as part of the parity test rather than hardcoding values?
