//! Keychain integration — service name derivation, read, and write.
//!
//! macOS: hex-encoded JSON via `security` CLI (CC compatibility).
//! Linux/Windows: direct JSON via `keyring` crate (CC doesn't read
//! keychain on these platforms).

use super::CredentialFile;
use crate::error::PlatformError;
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::warn;
use unicode_normalization::UnicodeNormalization;

/// Keychain account parameter (matches CC's usage).
const KEYCHAIN_ACCOUNT: &str = "credentials";

/// Derives the keychain service name for a given CC config directory.
///
/// Format: `Claude Code-credentials-{hash}` where `{hash}` is the
/// first 8 hex characters of SHA-256 of the NFC-normalized path.
pub fn service_name(config_dir: &Path) -> String {
    let normalized: String = config_dir.to_string_lossy().nfc().collect();
    let hash = Sha256::digest(normalized.as_bytes());
    let prefix = hex::encode(&hash[..4]); // 4 bytes = 8 hex chars
    format!("Claude Code-credentials-{prefix}")
}

/// Writes credentials to the system keychain (best-effort).
///
/// On macOS: hex-encodes JSON before writing (CC compatibility).
/// On other platforms: stores JSON directly.
///
/// Failures are logged but never propagated — keychain writes are
/// best-effort and must not block the critical path.
pub fn write(config_dir: &Path, creds: &CredentialFile) {
    let svc = service_name(config_dir);
    if let Err(e) = write_impl(&svc, creds) {
        warn!(
            service = %svc,
            error = %e,
            "keychain write failed (best-effort, non-fatal)"
        );
    }
}

/// Reads credentials from the system keychain.
///
/// On macOS: reads and hex-decodes the payload.
/// On other platforms: reads JSON directly.
///
/// Returns `None` if the keychain entry doesn't exist or can't be read.
pub fn read(config_dir: &Path) -> Option<CredentialFile> {
    let svc = service_name(config_dir);
    match read_impl(&svc) {
        Ok(creds) => Some(creds),
        Err(e) => {
            warn!(
                service = %svc,
                error = %e,
                "keychain read failed"
            );
            None
        }
    }
}

// ── macOS implementation ──────────────────────────────────────────────

/// Keychain operation timeout in seconds (matches v1.x `subprocess.run(..., timeout=3)`).
#[cfg(target_os = "macos")]
const KEYCHAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Runs a `security` CLI command with a 3-second timeout.
/// Returns stdout on success, PlatformError on failure or timeout.
#[cfg(target_os = "macos")]
fn run_security_command(args: &[&str]) -> Result<Vec<u8>, PlatformError> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut child = Command::new("security")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PlatformError::Keychain(format!("security command: {e}")))?;

    // Poll with 100ms intervals up to the timeout
    let poll_interval = std::time::Duration::from_millis(100);
    let deadline = std::time::Instant::now() + KEYCHAIN_TIMEOUT;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Child exited — read its output
                let mut stdout = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if status.success() {
                    return Ok(stdout);
                } else {
                    let mut stderr = String::new();
                    if let Some(mut err) = child.stderr.take() {
                        let _ = err.read_to_string(&mut stderr);
                    }
                    return Err(PlatformError::Keychain(format!(
                        "security command failed: {stderr}"
                    )));
                }
            }
            Ok(None) => {
                // Still running
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(PlatformError::Keychain(
                        "keychain operation timed out (3s)".into(),
                    ));
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => return Err(PlatformError::Keychain(format!("wait failed: {e}"))),
        }
    }
}

#[cfg(target_os = "macos")]
fn write_impl(service: &str, creds: &CredentialFile) -> Result<(), PlatformError> {
    let json = serde_json::to_string(creds)
        .map_err(|e| PlatformError::Keychain(format!("serialize: {e}")))?;
    let hex_payload = hex::encode(json.as_bytes());

    run_security_command(&[
        "add-generic-password",
        "-U",
        "-s",
        service,
        "-a",
        KEYCHAIN_ACCOUNT,
        "-w",
        &hex_payload,
    ])?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn read_impl(service: &str) -> Result<CredentialFile, PlatformError> {
    let stdout = run_security_command(&[
        "find-generic-password",
        "-s",
        service,
        "-a",
        KEYCHAIN_ACCOUNT,
        "-w",
    ])?;

    let hex_payload = String::from_utf8_lossy(&stdout).trim().to_string();
    let bytes = hex::decode(&hex_payload)
        .map_err(|e| PlatformError::Keychain(format!("hex decode: {e}")))?;
    let json = String::from_utf8(bytes)
        .map_err(|e| PlatformError::Keychain(format!("utf8: {e}")))?;

    serde_json::from_str(&json)
        .map_err(|e| PlatformError::Keychain(format!("json parse: {e}")))
}

// ── Linux/Windows stub (keyring crate integration deferred) ───────────

#[cfg(not(target_os = "macos"))]
fn write_impl(_service: &str, _creds: &CredentialFile) -> Result<(), PlatformError> {
    // Linux/Windows keychain integration requires the `keyring` crate.
    // CC does not read the keychain on these platforms, so this is
    // lower priority. For now, file-based storage is the primary path.
    warn!("keychain write not implemented on this platform");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn read_impl(_service: &str) -> Result<CredentialFile, PlatformError> {
    Err(PlatformError::Keychain(
        "keychain read not implemented on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_format() {
        let svc = service_name(Path::new("/Users/test/.claude/accounts/config-1"));
        assert!(svc.starts_with("Claude Code-credentials-"));
        assert_eq!(svc.len(), "Claude Code-credentials-".len() + 8);
    }

    #[test]
    fn service_name_deterministic() {
        let path = Path::new("/Users/test/.claude/accounts/config-1");
        let a = service_name(path);
        let b = service_name(path);
        assert_eq!(a, b);
    }

    #[test]
    fn service_name_different_for_different_paths() {
        let a = service_name(Path::new("/Users/test/.claude/accounts/config-1"));
        let b = service_name(Path::new("/Users/test/.claude/accounts/config-2"));
        assert_ne!(a, b);
    }

    #[test]
    fn service_name_nfc_normalization() {
        // NFC normalization: é as single codepoint vs e + combining accent
        let composed = service_name(Path::new("/tmp/caf\u{00e9}"));
        let decomposed = service_name(Path::new("/tmp/caf\u{0065}\u{0301}"));
        assert_eq!(composed, decomposed, "NFC normalization should produce same hash");
    }

    #[test]
    fn service_name_hex_is_lowercase() {
        let svc = service_name(Path::new("/tmp/test"));
        let hash_part = &svc["Claude Code-credentials-".len()..];
        assert!(
            hash_part.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hash should be lowercase hex: {hash_part}"
        );
    }
}
