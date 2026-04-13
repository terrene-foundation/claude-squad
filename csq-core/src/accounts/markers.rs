//! Account marker files — durable identity markers for CC sessions.
//!
//! `.csq-account` — written by csq during setup, contains account number.
//! `.current-account` — fast-path cache, written by snapshot_account().
//! `.live-pid` — PID of the CC process, used for snapshot caching.

use crate::error::CredentialError;
use crate::platform::fs::{atomic_replace, secure_file};
use crate::types::AccountNum;
use std::path::Path;

/// Reads the `.csq-account` marker from a config directory.
/// Returns None if the file doesn't exist or contains invalid content.
pub fn read_csq_account(config_dir: &Path) -> Option<AccountNum> {
    let path = config_dir.join(".csq-account");
    read_account_marker(&path)
}

/// Writes the `.csq-account` marker to a config directory.
pub fn write_csq_account(config_dir: &Path, account: AccountNum) -> Result<(), CredentialError> {
    let path = config_dir.join(".csq-account");
    write_account_marker(&path, account)
}

/// Reads the `.current-account` fast-path marker.
pub fn read_current_account(config_dir: &Path) -> Option<AccountNum> {
    let path = config_dir.join(".current-account");
    read_account_marker(&path)
}

/// Writes the `.current-account` fast-path marker.
pub fn write_current_account(
    config_dir: &Path,
    account: AccountNum,
) -> Result<(), CredentialError> {
    let path = config_dir.join(".current-account");
    write_account_marker(&path, account)
}

/// Reads the `.live-pid` file. Returns None if missing, invalid,
/// or if the path is a symlink. Refusing symlinks closes a
/// cross-handle PID forgery vector where a poisoned handle dir
/// could point `.live-pid` at another process's status file and
/// trick the daemon's sweep into treating a dead handle as live.
pub fn read_live_pid(config_dir: &Path) -> Option<u32> {
    read_pid_marker(config_dir, ".live-pid")
}

/// Reads the `.live-cc-pid` file, used on non-Unix platforms to
/// record the spawned CC child process PID. On Unix this file is
/// never written (exec replaces csq-cli with claude so there is a
/// single PID). The sweep treats the handle dir as live if EITHER
/// `.live-pid` or `.live-cc-pid` is alive, closing the Windows
/// crash-recovery window where csq-cli died but CC survived as an
/// orphaned child.
pub fn read_live_cc_pid(config_dir: &Path) -> Option<u32> {
    read_pid_marker(config_dir, ".live-cc-pid")
}

fn read_pid_marker(config_dir: &Path, name: &str) -> Option<u32> {
    let path = config_dir.join(name);
    // symlink_metadata does NOT follow symlinks; if the path is a
    // symlink we refuse rather than read through it.
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_file() => {}
        _ => return None,
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Writes the `.live-pid` file.
pub fn write_live_pid(config_dir: &Path, pid: u32) -> Result<(), CredentialError> {
    write_pid_marker(config_dir, ".live-pid", pid)
}

/// Writes the `.live-cc-pid` file. Only used on non-Unix — see
/// [`read_live_cc_pid`] for the rationale.
pub fn write_live_cc_pid(config_dir: &Path, pid: u32) -> Result<(), CredentialError> {
    write_pid_marker(config_dir, ".live-cc-pid", pid)
}

fn write_pid_marker(config_dir: &Path, name: &str, pid: u32) -> Result<(), CredentialError> {
    let path = config_dir.join(name);
    let tmp = crate::platform::fs::unique_tmp_path(&path);
    std::fs::write(&tmp, pid.to_string().as_bytes()).map_err(|e| CredentialError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    atomic_replace(&tmp, &path).map_err(|e| CredentialError::Io {
        path: path.clone(),
        source: std::io::Error::other(e.to_string()),
    })?;
    Ok(())
}

fn read_account_marker(path: &Path) -> Option<AccountNum> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn write_account_marker(path: &Path, account: AccountNum) -> Result<(), CredentialError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let tmp = crate::platform::fs::unique_tmp_path(path);
    std::fs::write(&tmp, account.to_string().as_bytes()).map_err(|e| CredentialError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    secure_file(&tmp).ok();
    atomic_replace(&tmp, path).map_err(|e| CredentialError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(e.to_string()),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_read_csq_account() {
        let dir = TempDir::new().unwrap();
        let account = AccountNum::try_from(5u16).unwrap();

        write_csq_account(dir.path(), account).unwrap();
        assert_eq!(read_csq_account(dir.path()), Some(account));
    }

    #[test]
    fn write_read_current_account() {
        let dir = TempDir::new().unwrap();
        let account = AccountNum::try_from(3u16).unwrap();

        write_current_account(dir.path(), account).unwrap();
        assert_eq!(read_current_account(dir.path()), Some(account));
    }

    #[test]
    fn read_missing_marker_returns_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_csq_account(dir.path()), None);
        assert_eq!(read_current_account(dir.path()), None);
    }

    #[test]
    fn read_invalid_marker_returns_none() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".csq-account"), "not-a-number").unwrap();
        assert_eq!(read_csq_account(dir.path()), None);
    }

    #[test]
    fn read_out_of_range_marker_returns_none() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".csq-account"), "0").unwrap();
        assert_eq!(read_csq_account(dir.path()), None);

        std::fs::write(dir.path().join(".csq-account"), "1000").unwrap();
        assert_eq!(read_csq_account(dir.path()), None);
    }

    #[test]
    fn write_read_live_pid() {
        let dir = TempDir::new().unwrap();
        write_live_pid(dir.path(), 12345).unwrap();
        assert_eq!(read_live_pid(dir.path()), Some(12345));
    }

    #[test]
    fn read_missing_pid_returns_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_live_pid(dir.path()), None);
    }
}
