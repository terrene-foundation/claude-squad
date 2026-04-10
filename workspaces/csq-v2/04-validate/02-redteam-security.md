# Red Team Report: v2.0 Security Analysis

## Date: 2026-04-10

## Agent: security-reviewer

## Scope: Security analysis, ADRs, credential flows, v1.x source, new Tauri attack surface

---

## CRITICAL (3)

### C1. Unix socket / named pipe has no authentication — local privilege escalation

ADR-005 specifies daemon listens on `/tmp/csq-{uid}.sock` with "no authentication required." On multi-user systems, any local process can connect and invoke `swap-account`, `refresh-token`, or `POST /api/login/{N}`. Socket must be in `$XDG_RUNTIME_DIR` (user-private) or equivalent, with per-session bearer token.

**Needed**: SEC-018 requirement for socket auth + location.

### C2. Auto-update has no signature verification — supply chain attack vector

`csq` lines 54-84 download from GitHub raw URLs over HTTPS with no signature or checksum verification. MITM at TLS termination (corporate proxy, DNS hijack) injects arbitrary code with access to all OAuth credentials. v2.0 Tauri desktop uses Ed25519 signed updates, but CLI-only mode has no equivalent.

**Needed**: SEC-019 requiring Ed25519 signature verification on all update paths.

### C3. OAuth callback on fixed port 8420 — port pre-binding attack

Malicious local app binds port 8420 before daemon, intercepts OAuth redirects. PKCE mitigates code exchange but attacker can deny service or present fake "login successful" page.

**Needed**: Use ephemeral port for OAuth callback, or validate bind success before presenting auth URL.

---

## HIGH (5)

### H1. WebView XSS surface not fully addressed

- DOM injection via account labels/emails if `{@html ...}` used
- IPC payload poisoning via crafted credential fields
- `style-src 'unsafe-inline'` allows CSS-based exfiltration

**Needed**: SEC-020 banning `{@html}` on untrusted data, remove `unsafe-inline` from style-src.

### H2. OAuth pending login state has no expiry

`OAuthLogin._pending_logins` stores state tokens indefinitely. Observed state parameters (browser history, proxy logs) can be replayed hours later.

**Needed**: SEC-021, 10-minute TTL on pending states.

### H3. Refresher has TOCTOU race (no file lock)

Dashboard refresher reads/compares/writes credentials without holding the per-account file lock. Between post-read and write, another process can write newer credentials that get overwritten.

**Needed**: REL-015 requiring daemon refresh to acquire file lock.

### H4. `csq setkey` passes API keys via environment variable

`CSQ_KEY="$key"` visible via `/proc/{pid}/environ`. In v2.0 Rust, read key from stdin directly.

**Needed**: SEC-022 banning API keys in env vars or CLI args.

### H5. `.gitignore` missing `config-*/` and `.credentials.json`

Developer debugging in repo directory could accidentally commit credentials.

---

## MEDIUM (6)

- M1: Auto-update writes `rotation-engine.py` non-atomically (curl -o, truncation on interrupt)
- M2: No rate limiting on daemon API (flood refresh = trigger Anthropic throttle)
- M3: `_pending_logins` dict unbounded (memory leak with secret material)
- M4: Windows named pipe collision on multi-session (name needs hash of accounts dir)
- M5: No `secrecy` crate usage in v1.x (tokens persist in Python memory allocator)
- M6: Swap + CC internal refresh race not documented in user flows

---

## LOW (4)

- L1: CLIENT_ID duplicated across 3 files
- L2: User-Agent impersonates claude-code/2.1.91
- L3: install.sh creates config-1 through 7 unconditionally
- L4: accounts.py logs file paths with username in warning messages

---

## PASSED CHECKS

The following were verified as correctly designed:

- No hardcoded real tokens in source
- No `shell=True` anywhere — all subprocess calls use array form
- Input validation via `_validate_account()` at every entry point
- Keychain service name always computed from SHA-256(NFC path), never from user input
- Atomic writes on ALL credential paths (temp+replace)
- File permissions `0o600` on credential files
- Fail-closed on keychain (3s timeout, non-fatal)
- Fail-closed on lock contention (skip and retry)
- Backsync/pullsync monotonicity guards with re-read inside lock
- Broker recovery pattern (iterate siblings, track tried RTs, restore on total failure)
- PKCE implementation cryptographically correct
- No full tokens in logs (prefix-only)
- Tauri IPC allowlist correctly specified
- CSP headers correctly specified (no unsafe-eval)
- Supply chain for Tauri desktop (Ed25519 + cargo-audit + cargo-vet)

## v1.x Security Lessons NOT Captured in v2.0 Spec

1. **journal/0005**: Credential attribution must use content match (refresh token), not marker files — not called out as explicit rule in v2.0
2. **journal/0013**: Windows handle truncation — Rust eliminates this but concurrent locking test should carry forward
3. **journal/0011**: "Stuck access tokens" (pass auth status but fail inference) — v2.0 needs `csq verify N` command
