# csq v2.0 — Gap Resolutions

Resolutions for the 10 spec gaps identified in `04-validate/03-redteam-filtered.md`. Each gap is resolved with enough detail to write code against — struct definitions, constants, algorithms, and edge cases.

**Source data**: Real credential, quota, and profile files from a live 8-account installation. v1.x source code. ADRs 001-006.

---

## GAP-1: Credential JSON Schema (RESOLVED)

### Real File Structure

`credentials/N.json` contains a single top-level key `claudeAiOauth` wrapping the OAuth token payload. This shape is defined by Claude Code, not csq — csq must preserve it exactly.

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1775726524877,
    "scopes": [
      "user:file_upload",
      "user:inference",
      "user:mcp_servers",
      "user:profile",
      "user:sessions:claude_code"
    ],
    "subscriptionType": "max",
    "rateLimitTier": "default_claude_max_20x"
  }
}
```

### Rust Struct

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level credential file. CC owns this schema — we must preserve
/// every field, including ones we don't recognize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialFile {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: OAuthPayload,

    /// Forward-compat: preserve unknown top-level keys.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// OAuth token payload within the credential file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthPayload {
    /// Bearer access token. Prefix: `sk-ant-oat01-`.
    #[serde(rename = "accessToken")]
    pub access_token: AccessToken,

    /// Single-use refresh token. Prefix: `sk-ant-ort01-`.
    #[serde(rename = "refreshToken")]
    pub refresh_token: RefreshToken,

    /// Expiry as Unix milliseconds (NOT seconds).
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,

    /// OAuth scopes granted. Preserved verbatim on refresh.
    pub scopes: Vec<String>,

    /// Subscription tier. Values observed: "max", "pro", "free".
    /// Preserved verbatim on refresh — never set by csq.
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,

    /// Rate-limit tier. Values observed: "default_claude_max_20x".
    /// Preserved verbatim on refresh — never set by csq.
    #[serde(rename = "rateLimitTier")]
    pub rate_limit_tier: Option<String>,

    /// Forward-compat: preserve unknown fields from CC updates.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
```

### Field Preservation Rule

On token refresh, the HTTP response from Anthropic returns `access_token`, `refresh_token`, `expires_in`, and sometimes `scope`. The response does NOT return `subscriptionType` or `rateLimitTier`. csq MUST merge the response into the existing payload:

```rust
fn merge_refresh_response(existing: &mut OAuthPayload, response: &RefreshResponse) {
    existing.access_token = response.access_token.clone();
    existing.refresh_token = response.refresh_token.clone();
    existing.expires_at = now_millis() + (response.expires_in * 1000);
    // subscriptionType, rateLimitTier, scopes, extra: PRESERVED from existing
}
```

### Critical Constants

| Constant         | Value                         | Source                                 |
| ---------------- | ----------------------------- | -------------------------------------- |
| `expiresAt` unit | Milliseconds (not seconds)    | Real file: `1775726524877` (13 digits) |
| Token lifetime   | ~5 hours (18000s)             | Observed from Anthropic responses      |
| Refresh window   | 2 hours (7200s) before expiry | ADR-006                                |

---

## GAP-2: Keychain Hex-Encoding Decision (RESOLVED)

### Problem

ADR-003 says "store JSON directly" via the `keyring` crate. CC reads the macOS Keychain using `security find-generic-password` and expects **hex-encoded JSON**. The v1.x code hex-encodes before writing:

```python
# v1.x: rotation-engine.py L797
hex_payload = json.dumps(data).encode().hex()
subprocess.run(["security", "add-generic-password", "-U",
    "-s", service, "-a", account, "-w", hex_payload], timeout=3)
```

CC's reader does:

```
hex_payload = security find-generic-password -s <service> -w
credentials = json.loads(bytes.fromhex(hex_payload))
```

If v2.0 uses `keyring` crate with direct JSON, CC cannot decode the keychain entry.

### Decision: Hex-encode on macOS, keyring crate on Linux/Windows

**macOS**: Use `security-framework` crate (direct Security.framework FFI, no subprocess) to write **hex-encoded JSON** to the keychain. This matches CC's expected format exactly. The `keyring` crate's generic API doesn't support hex encoding, so we bypass it on macOS.

**Linux**: Use `keyring` crate with `libsecret` backend. Store JSON directly. CC does not read the keychain on Linux (no `security` command), so there is no compatibility constraint.

**Windows**: Use `keyring` crate with Windows Credential Manager backend. Store JSON directly. Same reasoning as Linux.

### ADR-003 Amendment

Replace:

> No hex encoding: The crate handles serialization. Store JSON directly.

With:

> macOS: Hex-encode JSON before writing to Keychain (CC compatibility). Use `security-framework` crate for direct FFI. Linux/Windows: Store JSON directly via `keyring` crate. CC does not read the keychain on these platforms.

### Implementation

```rust
#[cfg(target_os = "macos")]
pub fn write_keychain(service: &str, account: &str, creds: &CredentialFile) -> Result<()> {
    let json = serde_json::to_string(creds)?;
    let hex = hex::encode(json.as_bytes());
    // security-framework: GenericPassword::set_password()
    security_framework::passwords::set_generic_password(service, account, hex.as_bytes())?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn write_keychain(service: &str, account: &str, creds: &CredentialFile) -> Result<()> {
    let json = serde_json::to_string(creds)?;
    let entry = keyring::Entry::new(service, account)?;
    entry.set_password(&json)?;
    Ok(())
}
```

### Keychain Service Name (unchanged from ADR-003)

```rust
pub fn keychain_service(config_dir: &Path) -> String {
    let normalized = config_dir.to_string_lossy().nfc().collect::<String>();
    let hash = sha256(normalized.as_bytes());
    let prefix = hex::encode(&hash[..4]); // first 8 hex chars = 4 bytes
    format!("Claude Code-credentials-{prefix}")
}
```

The account parameter for the keychain entry is `"credentials"` (matches CC's usage).

---

## GAP-3: Quota JSON Schema (RESOLVED)

### Real File Structure

`quota.json` has a single top-level `accounts` key mapping string account numbers to usage data:

```json
{
  "accounts": {
    "1": {
      "five_hour": {
        "used_percentage": 94,
        "resets_at": 1775714400
      },
      "seven_day": {
        "used_percentage": 100,
        "resets_at": 1775905200
      },
      "updated_at": 1775706062.924172
    }
  }
}
```

### Rust Structs

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaFile {
    pub accounts: HashMap<String, AccountQuota>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountQuota {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub updated_at: f64, // Unix seconds with fractional
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    /// 0-100 (can exceed 100 in edge cases due to floating point).
    pub used_percentage: f64,
    /// Unix timestamp (seconds) when this window resets.
    pub resets_at: u64,
}
```

### Expiry Logic

When loading quota state, clear expired windows:

```rust
impl AccountQuota {
    pub fn clear_expired(&mut self, now_secs: u64) {
        if let Some(ref w) = self.five_hour {
            if w.resets_at <= now_secs { self.five_hour = None; }
        }
        if let Some(ref w) = self.seven_day {
            if w.resets_at <= now_secs { self.seven_day = None; }
        }
    }
}
```

### Key Observations

- Account keys are **strings** (not integers) in JSON: `"1"`, `"8"`.
- `used_percentage` is a float (observed: `55.00000000000001` from Python floating point).
- `resets_at` is seconds (10-digit), NOT milliseconds. Contrast with credential `expiresAt` which is milliseconds.
- `updated_at` is seconds with fractional part (float64).

---

## GAP-4: Error Type Hierarchy (RESOLVED)

### Design Principles

1. `thiserror` at module boundaries (typed variants the caller can match on).
2. `anyhow` for internal propagation within modules.
3. `CsqError` as the top-level enum for CLI and Tauri command handlers.
4. Every variant carries enough context to produce a user-facing message.

### Top-Level Error

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CsqError {
    #[error("credential error: {0}")]
    Credential(#[from] CredentialError),

    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),

    #[error("broker error: {0}")]
    Broker(#[from] BrokerError),

    #[error("oauth error: {0}")]
    OAuth(#[from] OAuthError),

    #[error("daemon error: {0}")]
    Daemon(#[from] DaemonError),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
```

### Per-Module Errors

```rust
#[derive(Error, Debug)]
pub enum CredentialError {
    #[error("credential file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("corrupt credential file {path}: {reason}")]
    Corrupt { path: PathBuf, reason: String },

    #[error("invalid account number: {0}")]
    InvalidAccount(String),

    #[error("no credentials configured for account {0}")]
    NoCredentials(u16),

    #[error("io error on {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
}

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("lock contention on {path} (held by another process)")]
    LockContention { path: PathBuf },

    #[error("lock timeout after {timeout_ms}ms on {path}")]
    LockTimeout { path: PathBuf, timeout_ms: u64 },

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("process not found: PID {pid}")]
    ProcessNotFound { pid: u32 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("windows error: code {code}, {message}")]
    Win32 { code: u32, message: String },
}

#[derive(Error, Debug)]
pub enum BrokerError {
    #[error("refresh failed for account {account}: {reason}")]
    RefreshFailed { account: u16, reason: String },

    #[error("refresh token invalid for account {account} (re-login required)")]
    RefreshTokenInvalid { account: u16 },

    #[error("all siblings dead for account {account}")]
    AllSiblingsDead { account: u16 },

    #[error("recovery failed for account {account}: tried {tried} siblings")]
    RecoveryFailed { account: u16, tried: usize },
}

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("http error: {status} {body}")]
    Http { status: u16, body: String },

    #[error("state token expired (TTL {ttl_secs}s exceeded)")]
    StateExpired { ttl_secs: u64 },

    #[error("state token mismatch (CSRF)")]
    StateMismatch,

    #[error("PKCE verification failed")]
    PkceVerification,

    #[error("token exchange failed: {0}")]
    Exchange(String),
}

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("daemon not running (PID file: {pid_path})")]
    NotRunning { pid_path: PathBuf },

    #[error("daemon already running (PID {pid})")]
    AlreadyRunning { pid: u32 },

    #[error("socket connect failed: {path}")]
    SocketConnect { path: PathBuf },

    #[error("ipc timeout after {timeout_ms}ms")]
    IpcTimeout { timeout_ms: u64 },

    #[error("stale PID file (PID {pid} not alive)")]
    StalePidFile { pid: u32 },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("profile not found: {name}")]
    ProfileNotFound { name: String },

    #[error("invalid JSON in {path}: {reason}")]
    InvalidJson { path: PathBuf, reason: String },

    #[error("settings merge conflict in {key}")]
    MergeConflict { key: String },
}
```

### Tauri Command Mapping

For Tauri commands, `CsqError` maps to a string code + message:

```rust
impl From<CsqError> for String {
    fn from(e: CsqError) -> String {
        match &e {
            CsqError::Credential(CredentialError::NotFound { .. }) => format!("NOT_FOUND: {e}"),
            CsqError::Credential(CredentialError::InvalidAccount(_)) => format!("INVALID_INPUT: {e}"),
            CsqError::Broker(BrokerError::RefreshTokenInvalid { .. }) => format!("LOGIN_REQUIRED: {e}"),
            CsqError::OAuth(OAuthError::StateMismatch) => format!("CSRF_ERROR: {e}"),
            _ => format!("INTERNAL_ERROR: {e}"),
        }
    }
}
```

---

## GAP-5: Cargo Workspace Structure (RESOLVED)

### Layout

```
claude-squad/
├── Cargo.toml              # Workspace root
├── csq-core/               # Library crate: all business logic
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── credentials/    # store, validate, oauth, keychain
│       ├── accounts/       # identity, snapshot, discovery, profiles
│       ├── platform/       # fs, lock, process, win32
│       ├── rotation/       # swap, picker, auto
│       ├── daemon/         # broker, fanout, sync, refresher, poller
│       ├── quota/          # update, state, statusline, format
│       ├── providers/      # setkey, catalog, validate, repair, ollama
│       ├── session/        # isolation, settings, setup
│       └── error.rs        # CsqError hierarchy
├── csq-cli/                # Binary crate: CLI entry point
│   ├── Cargo.toml
│   └── src/
│       └── main.rs         # clap routing, subcommands
├── src-tauri/              # Tauri binary: desktop app
│   ├── Cargo.toml          # Depends on csq-core + tauri
│   ├── src/
│   │   ├── main.rs         # Tauri setup, window, tray
│   │   └── commands/       # Tauri IPC command handlers
│   ├── tauri.conf.json
│   └── capabilities/
├── src/                    # Svelte frontend (Vite-built, bundled into Tauri)
│   ├── lib/
│   │   ├── components/
│   │   ├── stores/
│   │   └── utils/
│   ├── App.svelte
│   └── main.ts
├── package.json            # Frontend build dependencies
└── vite.config.ts
```

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["csq-core", "csq-cli", "src-tauri"]

[workspace.package]
version = "2.0.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/terrene-foundation/claude-squad"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
hex = "0.4"
sha2 = "0.10"
unicode-normalization = "0.1"
reqwest = { version = "0.12", features = ["json"] }
clap = { version = "4", features = ["derive"] }
```

### csq-core/Cargo.toml

```toml
[package]
name = "csq-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tokio.workspace = true
tracing.workspace = true
hex.workspace = true
sha2.workspace = true
unicode-normalization.workspace = true
reqwest.workspace = true

# Platform-specific
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "3"

[target.'cfg(not(target_os = "macos"))'.dependencies]
keyring = { version = "3", features = ["crypto-rust"] }

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_System_Threading", "Win32_Foundation", "Win32_System_Diagnostics_ToolHelp"] }
```

### csq-cli/Cargo.toml

```toml
[package]
name = "csq-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "csq"
path = "src/main.rs"

[dependencies]
csq-core = { path = "../csq-core" }
clap.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true
```

### src-tauri/Cargo.toml

```toml
[package]
name = "csq-desktop"
version.workspace = true
edition.workspace = true

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
csq-core = { path = "../csq-core" }
tauri = { version = "2", features = ["tray-icon", "protocol-asset"] }
tauri-plugin-shell = "2"
tauri-plugin-updater = "2"
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
```

### Key Design Decisions

1. **csq-core has zero Tauri dependency.** All business logic is pure Rust. The Tauri binary depends on csq-core, not the reverse.
2. **csq-cli and src-tauri are both thin wrappers.** CLI routes clap subcommands to csq-core functions. Tauri routes IPC commands to csq-core functions.
3. **Single binary is achieved by feature flags in csq-cli**, not by merging crates. `csq-cli` with `--features desktop` links against Tauri. Without the feature, it is CLI-only (<5MB).
4. **src-tauri/ follows Tauri v2 conventions.** `tauri.conf.json`, `capabilities/`, and `src/main.rs` are in the standard Tauri locations. Tauri's build system finds them automatically.

---

## GAP-6: Auto-Rotation Spec (RESOLVED)

### Behavior

Auto-rotation is a daemon feature that automatically swaps the active account when the current account's quota is exhausted or critically low. It operates per-terminal (each CC instance has its own active account).

### Trigger Conditions

Auto-rotation triggers when ALL of these are true:

1. **5-hour usage >= threshold** (default: 95%). The 5-hour window is the actionable constraint — 7-day exhaustion is informational only.
2. **A better account exists**: `pick_best()` returns an account with 5-hour usage < threshold.
3. **Terminal is idle**: No CC API call in progress (detected by checking if CC is waiting for input, not streaming a response). This prevents mid-conversation swaps.
4. **Cooldown elapsed**: At least 5 minutes since last auto-rotation for this terminal. Prevents thrashing when all accounts are near the threshold.

### Configuration

Stored in `~/.claude/accounts/rotation.json`:

```json
{
  "auto_rotate": {
    "enabled": false,
    "threshold_percent": 95,
    "cooldown_secs": 300,
    "exclude_accounts": []
  }
}
```

- **Disabled by default.** Users opt in with `csq config set auto_rotate.enabled true`.
- `exclude_accounts`: list of account numbers that should never be auto-rotated TO (e.g., a personal account the user wants to keep for manual use).

### Daemon Implementation

```
every 30 seconds:
  for each active terminal (has a .live-pid with alive process):
    account = read .csq-account marker
    quota = get quota for account from cache
    if quota.five_hour.used_percentage >= threshold:
      best = pick_best(exclude=[account] + config.exclude_accounts)
      if best is not None and best.five_hour.used_percentage < threshold:
        if terminal_is_idle(pid):
          if cooldown_elapsed(terminal, now):
            swap_to(terminal_config_dir, best.account)
            emit event: "auto-rotated terminal {config_dir} from #{account} to #{best}"
            update cooldown timestamp
```

### CLI Fallback

Without a daemon, auto-rotation runs synchronously during the statusline hook (same as v1.x `auto_rotate()` with `--force`). It checks the same conditions but runs at most once per statusline render.

### Per-Terminal vs Per-Account

Auto-rotation is **per-terminal**: it swaps the credentials in a specific `config-N/.credentials.json`. It does NOT affect other terminals using the same account. If 3 terminals are on account #1 and it hits the threshold, the daemon rotates each terminal independently (they may end up on different accounts).

---

## GAP-7: profiles.json Schema (RESOLVED)

### Real File Structure

```json
{
  "accounts": {
    "1": {
      "email": "user@example.com",
      "method": "oauth"
    },
    "8": {
      "email": "user@other.com",
      "method": "oauth"
    }
  }
}
```

### Rust Struct

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub accounts: HashMap<String, AccountProfile>,

    /// Forward-compat: preserve unknown top-level keys.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountProfile {
    /// Email address associated with the account.
    pub email: String,

    /// Authentication method. Known values: "oauth", "api_key".
    pub method: String,

    /// Forward-compat: preserve unknown fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
```

### Usage Patterns

- **Write**: After `csq login N` completes, write `{email, method: "oauth"}` under `accounts.N`. Preserve existing entries for other accounts.
- **Read**: Account discovery reads `profiles.json` to resolve email for each `credentials/N.json`. Missing profile = "unknown" email, not an error.
- **3P accounts**: When `csq setkey` creates a provider profile, it writes `{email: "apikey", method: "api_key"}` to distinguish from OAuth accounts. The email field is a display label, not a real email.

---

## GAP-8: Windows WAIT_ABANDONED Handling (RESOLVED)

### Problem

When a process holding a Windows named mutex terminates without releasing it, `WaitForSingleObject` returns `WAIT_ABANDONED` (0x00000080) instead of `WAIT_OBJECT_0`. v1.x treats this as a failure (returns `None` from `_try_lock_file()`), causing permanent lock for the affected resource until reboot.

### Decision: Treat WAIT_ABANDONED as "Acquired With Warning"

```rust
#[cfg(windows)]
pub fn try_lock(name: &str) -> Result<Option<MutexGuard>, PlatformError> {
    let handle = unsafe { CreateMutexW(null(), FALSE, name.encode_utf16()) };
    if handle.is_null() {
        return Err(PlatformError::Win32 {
            code: GetLastError(),
            message: "CreateMutexW failed".into(),
        });
    }

    match unsafe { WaitForSingleObject(handle, 0) } {
        WAIT_OBJECT_0 => Ok(Some(MutexGuard { handle })),
        WAIT_ABANDONED => {
            tracing::warn!(
                mutex = name,
                "acquired abandoned mutex — previous holder crashed without release"
            );
            // Treat as acquired. The mutex is now owned by this thread.
            // The protected resource (credential file, quota file) may be
            // in an inconsistent state — callers should re-read and validate.
            Ok(Some(MutexGuard { handle }))
        }
        WAIT_TIMEOUT => Ok(None), // Held by another live process
        _ => Err(PlatformError::Win32 {
            code: GetLastError(),
            message: "WaitForSingleObject failed".into(),
        }),
    }
}
```

### Caller Responsibility

When `try_lock()` returns `Some(guard)` after `WAIT_ABANDONED`:

1. **Re-read the protected file** before assuming its contents are valid.
2. **Validate** the file (parse JSON, check `expiresAt` makes sense).
3. If the file is corrupt (truncated write from the crashed process), **restore from the other copy** (canonical ↔ live).

This responsibility already exists in the broker and backsync code — both re-read inside the lock. No additional logic needed.

### Platform Locking Spec Amendment

Add to the platform locking section in the scope matrix:

> **Windows WAIT_ABANDONED**: Treated as successful acquisition with a warning log. The mutex is owned by the acquiring thread. Callers MUST re-read and validate protected resources after acquiring an abandoned mutex, as the previous holder may have crashed mid-write.

---

## GAP-9: Daemon Detection Protocol (RESOLVED)

### PID File Location

| Platform | Path                                                                              |
| -------- | --------------------------------------------------------------------------------- |
| macOS    | `~/.claude/accounts/csq-daemon.pid`                                               |
| Linux    | `$XDG_RUNTIME_DIR/csq-daemon.pid` (fallback: `~/.claude/accounts/csq-daemon.pid`) |
| Windows  | `%LOCALAPPDATA%\csq\csq-daemon.pid`                                               |

The PID file contains a single line: the daemon's PID as a decimal integer.

### Socket Path

| Platform | Path                                                          |
| -------- | ------------------------------------------------------------- |
| macOS    | `~/.claude/accounts/csq.sock`                                 |
| Linux    | `$XDG_RUNTIME_DIR/csq.sock` (fallback: `/tmp/csq-{uid}.sock`) |
| Windows  | `\\.\pipe\csq-{username}`                                     |

### Liveness Check Order

When a CLI command needs to determine if the daemon is running:

```
1. Read PID file
   ├── File missing → daemon not running → use direct mode
   └── File exists → parse PID
       ├── Parse error → stale file → delete, use direct mode
       └── PID parsed
           2. Check PID alive (is_pid_alive)
              ├── Dead → stale PID file → delete PID file + socket, use direct mode
              └── Alive
                  3. Connect to socket (100ms timeout)
                     ├── Connect refused → process is alive but not the daemon
                     │   (another process reused the PID) → delete PID file, use direct mode
                     ├── Connect timeout → daemon is overloaded → use direct mode with warning
                     └── Connected
                         4. Send health check: GET /api/health (200ms timeout)
                            ├── Timeout or error → daemon is unhealthy → use direct mode with warning
                            └── 200 OK → daemon is healthy → delegate to daemon
```

### Timeouts

| Check          | Timeout | Rationale                                                        |
| -------------- | ------- | ---------------------------------------------------------------- |
| Socket connect | 100ms   | If the daemon can't accept connections in 100ms, it's overloaded |
| Health check   | 200ms   | Single JSON response; if this takes longer, daemon is stuck      |
| IPC request    | 1000ms  | Normal operational requests (status, swap delegation)            |
| Statusline IPC | 50ms    | Hard deadline — statusline must render fast or fall back         |

### Fallback Behavior

Direct mode (no daemon) is always available. The CLI silently falls back:

- **No warning** for missing PID file (daemon simply isn't running).
- **Warning on stderr** for stale PID/socket (cleanup happened).
- **Warning on stderr** for timeout (daemon is alive but slow — user may want to restart it).

### Daemon Startup Sequence

```
1. Check for existing PID file
   ├── Exists + PID alive + socket responsive → exit with "daemon already running (PID N)"
   └── Exists + stale → clean up PID file + socket
2. Bind socket
   ├── EADDRINUSE → another process holds the socket → exit with error
   └── Success → socket ready
3. Write PID file (atomic: temp file + rename)
4. Start subsystems: refresher, poller, cache, HTTP API
5. Log: "daemon started (PID N, socket {path})"
```

### Graceful Shutdown

On SIGTERM/SIGINT (Unix) or console control event (Windows):

1. Stop accepting new connections
2. Complete in-flight requests (5s deadline)
3. Stop refresher and poller
4. Remove socket file
5. Remove PID file
6. Exit 0

---

## GAP-10: OAuth State Token TTL (RESOLVED)

### Problem

The OAuth PKCE flow generates a random `state` parameter stored in a HashMap. Without TTL, abandoned login attempts (user closes browser) leak entries indefinitely.

### Design

```rust
use std::collections::HashMap;
use std::time::Instant;

pub struct OAuthStateStore {
    /// Pending states: state_token -> (code_verifier, created_at)
    states: HashMap<String, PendingState>,
}

struct PendingState {
    code_verifier: String,
    created_at: Instant,
}

const STATE_TTL: Duration = Duration::from_secs(600); // 10 minutes
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60); // sweep every minute
```

### TTL: 10 Minutes

- OAuth login flow takes <2 minutes in the normal case (user clicks, browser opens, user approves, callback fires).
- 10 minutes provides generous margin for slow connections or distracted users.
- Any state token older than 10 minutes is expired and will be rejected at the callback.

### Lifecycle

```
1. User clicks "Login" → generate state + code_verifier
2. Store in HashMap: state -> PendingState { code_verifier, created_at: Instant::now() }
3. Redirect to Anthropic authorize URL with state + code_challenge
4. Wait for callback...

On callback (/oauth/callback?code=X&state=Y):
  a. Look up Y in HashMap
     ├── Not found → reject (expired or CSRF)
     └── Found → check TTL
         ├── Expired (now - created_at > 10min) → remove entry, reject with OAuthError::StateExpired
         └── Valid → consume entry (remove from map), exchange code using code_verifier
```

### Cleanup

A background task runs every 60 seconds to sweep expired entries:

```rust
async fn cleanup_loop(store: Arc<Mutex<OAuthStateStore>>) {
    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        let mut store = store.lock().await;
        let now = Instant::now();
        store.states.retain(|_, pending| {
            now.duration_since(pending.created_at) < STATE_TTL
        });
    }
}
```

### Bounded Map

As a defense-in-depth measure, the HashMap is bounded to 100 entries. If a 101st login is attempted while 100 are pending (which should never happen in normal operation), the oldest entry is evicted:

```rust
impl OAuthStateStore {
    const MAX_PENDING: usize = 100;

    pub fn insert(&mut self, state: String, code_verifier: String) {
        if self.states.len() >= Self::MAX_PENDING {
            // Evict oldest
            if let Some(oldest_key) = self.states.iter()
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                self.states.remove(&oldest_key);
            }
        }
        self.states.insert(state, PendingState {
            code_verifier,
            created_at: Instant::now(),
        });
    }
}
```

### Single-Use Guarantee

State tokens are **consumed on use** (removed from the HashMap after successful lookup). This prevents replay attacks where a captured callback URL is resubmitted.
