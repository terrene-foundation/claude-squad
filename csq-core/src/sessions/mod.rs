//! Live Claude Code session discovery.
//!
//! Enumerates running `claude` processes under the current UID and
//! extracts, per process:
//!
//! 1. `CLAUDE_CONFIG_DIR` from the environment — this tells us
//!    which `~/.claude/accounts/config-N/` the session is bound to.
//! 2. The process `cwd` — which working directory the user started
//!    the session in (how they recognize "terminal #5").
//! 3. Start time — how long ago the session launched.
//!
//! From `CLAUDE_CONFIG_DIR` the caller can cross-reference the
//! dir's current active account, quota state, and credentials.
//!
//! ### Why this exists
//!
//! The tray quick-swap heuristic targets "the most recently modified
//! config-N", which works most of the time but is invisible: users
//! can't see which terminal is bound to which `config-N` until they
//! run `lsof` by hand. When a user has 8 accounts across 15 terminal
//! windows and terminal #5 hits a rate limit, they need:
//!
//! - to know *that it was terminal #5*, and
//! - to swap **only that terminal's** config dir to a fresh account
//!   without disturbing the other 14.
//!
//! This module provides the data. The desktop sessions view renders
//! it; the Tauri `swap_to_dir` command does the targeted swap.
//!
//! ### Platform strategy
//!
//! - **macOS** — `ps -E -o pid=,command=` dumps the environ inline
//!   for processes owned by the current UID. We parse the line,
//!   peel the command, and walk the remaining `KEY=VALUE` pairs.
//!   `lsof -a -p <pid> -d cwd -Fn` gives the cwd (can't rely on
//!   `ps -o cwd=` because macOS omits it).
//! - **Linux** — `/proc/<pid>/environ` is NUL-separated and readable
//!   by the process owner without root. `readlink /proc/<pid>/cwd`
//!   gives the cwd. `/proc/<pid>/stat` gives the start time.
//! - **Windows** — stub: returns an empty vector. Reading another
//!   process's environ on Windows requires
//!   `NtQueryInformationProcess` + `PEB` walking which needs unsafe
//!   code and careful version gating. Deferred once macOS/Linux are
//!   validated on real Windows targets.
//!
//! ### Privacy
//!
//! We filter to processes owned by the **current UID**. `ps -E`
//! already enforces this on macOS for non-root callers; on Linux
//! `/proc/<pid>/environ` returns EACCES on cross-UID reads.
//!
//! ### Filtering
//!
//! A process is a "CC session" iff:
//! 1. Its command's first token (argv\[0\] basename) is `claude`.
//! 2. Its environment contains `CLAUDE_CONFIG_DIR`.
//!
//! The command filter drops child processes that inherit
//! `CLAUDE_CONFIG_DIR` from their parent (pyright-langserver,
//! node MCP servers, etc.) — we only want one row per top-level
//! `claude` process.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A single discovered live Claude Code session.
///
/// All fields are derived from OS process state; none are read
/// from the csq credential store or quota file. Callers that want
/// quota/account data should cross-reference `config_dir` via
/// `accounts::discovery` + `quota::state`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    /// OS process ID.
    pub pid: u32,
    /// Working directory at process creation (read once; not live).
    pub cwd: PathBuf,
    /// Value of `CLAUDE_CONFIG_DIR` in the process's environment.
    pub config_dir: PathBuf,
    /// Account number extracted from `config_dir` (`config-<N>`).
    /// `None` if the dir doesn't match the expected shape.
    pub account_id: Option<u16>,
    /// Unix seconds since epoch when the process started. `None`
    /// if the platform couldn't report it.
    pub started_at: Option<u64>,
}

impl SessionInfo {
    /// Derives the account number from a `config-N` directory name.
    /// Returns `None` for any other shape.
    pub(crate) fn extract_account_id(config_dir: &std::path::Path) -> Option<u16> {
        let name = config_dir.file_name()?.to_str()?;
        let num_str = name.strip_prefix("config-")?;
        let num: u16 = num_str.parse().ok()?;
        if (1..=999).contains(&num) {
            Some(num)
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::list as list_impl;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::list as list_impl;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::list as list_impl;

/// Discover live CC sessions for the current user.
///
/// Silently skips processes whose env or cwd cannot be read — most
/// such failures mean the process has exited between enumeration
/// and per-process inspection. A completely broken platform
/// backend returns an empty vector, never panics.
pub fn list() -> Vec<SessionInfo> {
    list_impl()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extract_account_id_valid() {
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("/a/b/config-5")),
            Some(5)
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-999")),
            Some(999)
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-1")),
            Some(1)
        );
    }

    #[test]
    fn extract_account_id_rejects_out_of_range() {
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-0")),
            None
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-1000")),
            None
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-99999")),
            None
        );
    }

    #[test]
    fn extract_account_id_rejects_bad_shape() {
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-abc")),
            None
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("config-")),
            None
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("other-5")),
            None
        );
        assert_eq!(
            SessionInfo::extract_account_id(&PathBuf::from("/a/b/")),
            None
        );
    }

    #[test]
    fn list_does_not_panic() {
        // Smoke test — the important invariant is that `list()`
        // never panics on a live system, even when the platform
        // backend encounters unexpected state (permission errors,
        // short-lived child processes, missing /proc entries).
        let _ = list();
    }
}
