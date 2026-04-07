# Security Rules

## Scope

Applies to all code in claude-squad, with particular attention to the
OAuth credential flow, keychain writes, and atomic file handling in
`rotation-engine.py` and `csq`.

## MUST Rules

### 1. No Hardcoded Secrets

Credentials, tokens, and keys must come from the environment, `.env`, or
the OAuth flow. Never committed literals.

```
BAD:  api_key = "sk-ant-..."
GOOD: api_key = os.environ["ANTHROPIC_API_KEY"]
```

### 2. No Secrets in Logs or Error Messages

Access tokens, refresh tokens, and keychain payloads must NOT be logged.
Print prefixes/suffixes only when diagnostics need them:

```
BAD:  print(f"token: {access_token}")
GOOD: print(f"token: {access_token[:8]}...{access_token[-4:]}")
```

### 3. Input Validation on Account Numbers

Any value destined for `credentials/{N}.json`, config-dir path construction,
or keychain service name MUST be validated via `_validate_account()`
(range 1..MAX_ACCOUNTS, digits only). This blocks path traversal and
keychain-namespace injection.

### 4. Atomic Writes for Credential Files

`.credentials.json`, `credentials/N.json`, and marker files (`.csq-account`,
`.current-account`, `.quota-cursor`) MUST be written via `_atomic_replace`
(temp file → `os.replace`). Partial writes during a crash must not corrupt
a running CC's credential state.

### 5. File Permissions on Credential Files

After writing a credential file, call `_secure_file()` to set `0o600`.
On Windows, this is a no-op (handled by the filesystem ACL default).

### 6. Fail-Closed on Keychain/Lock Contention

Keychain writes (`security add-generic-password`) and file locks can hang
under concurrent load. Every call that touches them MUST use a short
timeout (3 seconds) and fall through safely. Never block a statusline
render waiting for the keychain.

## MUST NOT Rules

### 1. No `shell=True` on User-Influenced Input

`subprocess.run([...])` with an array — never `shell=True` with string
interpolation. Path components must never reach a shell.

### 2. No `.env` or `credentials/` in Git

`.gitignore` MUST list:

- `.env`
- `credentials/`
- `config-*/`
- `.credentials.json`

If any of these were ever committed, history rewrite is required.

### 3. No Global Keychain Writes Under User-Supplied Service Names

The keychain service name is derived from the hashed config dir path via
`_keychain_service()`. Never accept a service name from CLI or env input
directly.

## Cross-References

- `no-stubs.md` — no silent fallbacks that hide security errors
- `zero-tolerance.md` — pre-existing security issues must be fixed
