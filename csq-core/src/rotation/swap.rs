//! Account swap — copy canonical credentials into a config dir.
//!
//! `swap_to(target)` reads `credentials/N.json` and writes it to the
//! target config dir's `.credentials.json`, along with `.csq-account`
//! and `.current-account` markers. Never calls the refresh endpoint —
//! uses cached credentials only.
//!
//! # Locking
//!
//! The canonical read + live write runs under the per-account
//! `credentials/N.refresh-lock` — the same lock that
//! `broker::check::broker_check` acquires before refreshing. This
//! prevents **C5 (swap_to lock race)**: without the lock, a swap
//! could read a stale canonical file that the refresher is about to
//! overwrite, resulting in the live `.credentials.json` containing
//! a token that has already been rotated.
//!
//! The lock is **blocking** (`lock_file`) rather than try-lock
//! because swap is user-initiated — the user expects it to succeed,
//! not skip. If the refresher holds the lock (typically <1s during
//! a token refresh), swap waits until the refresh completes and
//! then reads the fresh canonical.
//!
//! Markers and keychain writes happen outside the lock — they don't
//! race with the refresher and holding the lock longer than needed
//! delays subsequent refresh ticks.

use crate::accounts::markers;
use crate::credentials::{self, file};
use crate::error::{CredentialError, CsqError};
use crate::platform::lock;
use crate::types::AccountNum;
use std::path::Path;
use tracing::{debug, warn};

/// Swaps the active account in a config directory.
///
/// Reads canonical credentials for `target`, writes them to
/// `config_dir/.credentials.json` (atomic), and updates markers.
///
/// Preserves `.quota-cursor` (NOT deleted during swap).
/// Best-effort keychain write.
pub fn swap_to(
    base_dir: &Path,
    config_dir: &Path,
    target: AccountNum,
) -> Result<SwapResult, CsqError> {
    let canonical_path = file::canonical_path(base_dir, target);

    // Acquire the per-account refresh lock. This is the same lock
    // that broker_check takes (via try_lock_file) before refreshing
    // credentials. Holding it ensures we read a consistent canonical
    // file — either pre- or post-refresh, never mid-write.
    let lock_path = canonical_path.with_extension("refresh-lock");
    let _guard = lock::lock_file(&lock_path)?;

    let creds = credentials::load(&canonical_path)?;

    let live_path = config_dir.join(".credentials.json");
    credentials::save(&live_path, &creds)?;

    // Verify by reading back
    let verify = credentials::load(&live_path).map_err(|e| {
        warn!(error = %e, "swap verification read failed");
        e
    })?;
    if verify.claude_ai_oauth.access_token.expose_secret()
        != creds.claude_ai_oauth.access_token.expose_secret()
    {
        return Err(CsqError::Credential(CredentialError::Corrupt {
            path: live_path.clone(),
            reason: "verification: access token mismatch after write".into(),
        }));
    }

    // Drop the lock before markers + keychain writes. These do not
    // race with the refresher and we want to minimize lock duration
    // so the refresher's next tick isn't delayed.
    drop(_guard);

    // Update markers
    markers::write_csq_account(config_dir, target)?;
    markers::write_current_account(config_dir, target)?;

    // Best-effort keychain write
    credentials::keychain::write(config_dir, &creds);

    debug!(account = %target, "swap complete");
    Ok(SwapResult {
        account: target,
        expires_at_ms: creds.claude_ai_oauth.expires_at,
    })
}

/// Result of a successful swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapResult {
    pub account: AccountNum,
    pub expires_at_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::{CredentialFile, OAuthPayload};
    use crate::types::{AccessToken, RefreshToken};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_creds(access: &str, refresh: &str) -> CredentialFile {
        CredentialFile {
            claude_ai_oauth: OAuthPayload {
                access_token: AccessToken::new(access.into()),
                refresh_token: RefreshToken::new(refresh.into()),
                expires_at: 9999999999999,
                scopes: vec![],
                subscription_type: None,
                rate_limit_tier: None,
                extra: HashMap::new(),
            },
            extra: HashMap::new(),
        }
    }

    #[test]
    fn swap_to_writes_all_files() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config-2");
        std::fs::create_dir_all(&config).unwrap();
        let target = AccountNum::try_from(3u16).unwrap();

        // Set up canonical
        let creds = make_creds("at-3", "rt-3");
        credentials::save(&file::canonical_path(dir.path(), target), &creds).unwrap();

        let result = swap_to(dir.path(), &config, target).unwrap();
        assert_eq!(result.account, target);

        // Live file written
        let live = credentials::load(&config.join(".credentials.json")).unwrap();
        assert_eq!(live.claude_ai_oauth.access_token.expose_secret(), "at-3");

        // Markers written
        assert_eq!(markers::read_csq_account(&config), Some(target));
        assert_eq!(markers::read_current_account(&config), Some(target));
    }

    #[test]
    fn swap_preserves_quota_cursor() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config-1");
        std::fs::create_dir_all(&config).unwrap();
        let target = AccountNum::try_from(1u16).unwrap();

        // Pre-existing quota cursor
        let cursor_path = config.join(".quota-cursor");
        std::fs::write(&cursor_path, "existing-cursor-hash").unwrap();

        // Set up canonical and swap
        let creds = make_creds("at-1", "rt-1");
        credentials::save(&file::canonical_path(dir.path(), target), &creds).unwrap();
        swap_to(dir.path(), &config, target).unwrap();

        // Cursor must still exist
        assert!(cursor_path.exists());
        assert_eq!(
            std::fs::read_to_string(&cursor_path).unwrap(),
            "existing-cursor-hash"
        );
    }

    #[test]
    fn swap_fails_if_canonical_missing() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config-1");
        std::fs::create_dir_all(&config).unwrap();
        let target = AccountNum::try_from(9u16).unwrap();

        let result = swap_to(dir.path(), &config, target);
        assert!(result.is_err());
    }

    /// Regression test for C5 (swap_to lock race).
    ///
    /// Simulates the race condition: a "refresher" thread holds the
    /// per-account refresh-lock while overwriting the canonical
    /// credential file. `swap_to` must block until the lock is
    /// released and read the FRESH canonical, not the stale one.
    #[test]
    fn swap_waits_for_refresh_lock_c5_regression() {
        use crate::platform::lock;
        use std::sync::{Arc, Barrier};
        use std::thread;

        let dir = TempDir::new().unwrap();
        let config = dir.path().join("config-3");
        std::fs::create_dir_all(&config).unwrap();
        let target = AccountNum::try_from(3u16).unwrap();

        // Install initial (stale) credentials.
        let stale = make_creds("stale-token", "rt-3");
        credentials::save(&file::canonical_path(dir.path(), target), &stale).unwrap();

        let canonical = file::canonical_path(dir.path(), target);
        let lock_path = canonical.with_extension("refresh-lock");

        // Barrier: ensures the "refresher" thread holds the lock
        // before swap_to starts.
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = Arc::clone(&barrier);
        let lock_path_clone = lock_path.clone();
        let canonical_clone = canonical.clone();

        let refresher = thread::spawn(move || {
            // 1. Acquire the lock (simulating broker_check).
            let _guard = lock::lock_file(&lock_path_clone).unwrap();
            // 2. Signal swap thread that the lock is held.
            barrier_clone.wait();
            // 3. Simulate refresh work (overwrite canonical).
            let fresh = make_creds("fresh-token", "rt-3-new");
            credentials::save(&canonical_clone, &fresh).unwrap();
            // 4. Short delay to ensure swap_to is blocked on the lock.
            thread::sleep(std::time::Duration::from_millis(50));
            // 5. Drop guard — releases the lock so swap_to can proceed.
        });

        // Wait until the "refresher" holds the lock.
        barrier.wait();

        // swap_to should block on the lock, then read the FRESH
        // canonical that the refresher wrote.
        let result = swap_to(dir.path(), &config, target).unwrap();
        assert_eq!(result.account, target);

        // The live file must contain the FRESH token, not the stale one.
        let live = credentials::load(&config.join(".credentials.json")).unwrap();
        assert_eq!(
            live.claude_ai_oauth.access_token.expose_secret(),
            "fresh-token",
            "C5 regression: swap_to must read post-refresh canonical, not stale"
        );

        refresher.join().unwrap();
    }
}
