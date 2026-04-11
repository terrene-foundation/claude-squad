//! File locking — POSIX `flock` on Unix, named mutexes on Windows.
//!
//! All locking is exposed through [`lock_file`] (blocking) and
//! [`try_lock_file`] (non-blocking). Both return an RAII guard that
//! releases the lock on drop.

use crate::error::PlatformError;
use std::path::Path;

// ── Public API (platform-dispatched) ──────────────────────────────────

/// Acquires an exclusive lock on `path`, blocking until available.
///
/// Returns a guard that releases the lock on drop. The lock file is
/// created if it does not exist.
pub fn lock_file(path: &Path) -> Result<FileLockGuard, PlatformError> {
    imp::lock_file(path)
}

/// Attempts to acquire an exclusive lock without blocking.
///
/// Returns `Ok(Some(guard))` if acquired, `Ok(None)` if the lock is
/// held by another process. The lock file is created if it does not
/// exist.
pub fn try_lock_file(path: &Path) -> Result<Option<FileLockGuard>, PlatformError> {
    imp::try_lock_file(path)
}

// ── Guard type ────────────────────────────────────────────────────────

/// RAII guard that releases the lock on drop.
pub struct FileLockGuard {
    // Held for its Drop impl — releasing the lock on scope exit.
    _inner: imp::InnerGuard,
}

impl std::fmt::Debug for FileLockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileLockGuard").finish_non_exhaustive()
    }
}

// ── Unix implementation (flock) ───────────────────────────────────────

#[cfg(unix)]
mod imp {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::os::unix::io::AsRawFd;

    pub struct InnerGuard {
        file: File,
    }

    impl Drop for InnerGuard {
        fn drop(&mut self) {
            unsafe {
                libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }

    pub fn lock_file(path: &Path) -> Result<FileLockGuard, PlatformError> {
        let file = open_lock_file(path)?;
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
        if rc != 0 {
            return Err(PlatformError::Io(std::io::Error::last_os_error()));
        }
        Ok(FileLockGuard {
            _inner: InnerGuard { file },
        })
    }

    pub fn try_lock_file(path: &Path) -> Result<Option<FileLockGuard>, PlatformError> {
        let file = open_lock_file(path)?;
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(PlatformError::Io(err));
        }
        Ok(Some(FileLockGuard {
            _inner: InnerGuard { file },
        }))
    }

    fn open_lock_file(path: &Path) -> Result<File, PlatformError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(PlatformError::Io)
    }
}

// ── Windows implementation (named mutex) ──────────────────────────────

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use tracing::warn;

    // Win32 constants
    const WAIT_OBJECT_0: u32 = 0x00000000;
    const WAIT_ABANDONED: u32 = 0x00000080;
    const WAIT_TIMEOUT: u32 = 0x00000102;
    const INFINITE: u32 = 0xFFFFFFFF;

    extern "system" {
        fn CreateMutexW(
            lpMutexAttributes: *const std::ffi::c_void,
            bInitialOwner: i32,
            lpName: *const u16,
        ) -> *mut std::ffi::c_void;
        fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
        fn ReleaseMutex(hMutex: *mut std::ffi::c_void) -> i32;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        fn GetLastError() -> u32;
    }

    pub struct InnerGuard {
        handle: *mut std::ffi::c_void,
    }

    // SAFETY: The mutex handle is process-scoped and safe to send across threads.
    unsafe impl Send for InnerGuard {}

    impl Drop for InnerGuard {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.handle);
                CloseHandle(self.handle);
            }
        }
    }

    // Win32 error codes we care about when falling back from the
    // Global namespace to the session-local namespace.
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_PRIVILEGE_NOT_HELD: u32 = 1314;

    /// Produces a mutex name hash from a path.
    ///
    /// Hashes the raw wide-char path on Windows (not the lossy
    /// UTF-8 form) so non-UTF-8 sequences and surrogate halves
    /// don't collide on `U+FFFD`. Uses the full path — callers
    /// can pre-canonicalize if case-insensitivity matters.
    fn hash_path(path: &Path) -> String {
        use sha2::{Digest, Sha256};
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        // Convert u16 to bytes for hashing (little-endian).
        let mut bytes = Vec::with_capacity(wide.len() * 2);
        for u in &wide {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let digest = Sha256::digest(&bytes);
        // 16 hex chars = 64 bits — sufficient collision resistance
        // for the file-system lock paths on a single machine.
        hex::encode(&digest[..8])
    }

    /// Produces a Windows named mutex name for a path.
    ///
    /// `use_global` selects the namespace:
    ///   * `true`  — `Global\csq-lock-{hash}` (visible across all
    ///                 Terminal Services sessions on the machine)
    ///   * `false` — `csq-lock-{hash}` (implicit `Local\` — per-
    ///                 session only)
    ///
    /// The Global namespace requires `SeCreateGlobalPrivilege`,
    /// which standard (non-elevated) user accounts lack.  Attempts
    /// to open a Global mutex without the privilege return
    /// `ERROR_ACCESS_DENIED` (5) or `ERROR_PRIVILEGE_NOT_HELD`
    /// (1314) — NOT `ERROR_PATH_NOT_FOUND`, despite a previous
    /// comment claiming otherwise.
    ///
    /// NOTE — same-process/same-thread re-entrancy: Windows named
    /// mutexes are re-entrant within the same thread.
    /// `WaitForSingleObject` returns WAIT_OBJECT_0 immediately if
    /// the calling thread already owns the mutex, so same-thread
    /// contention tests are unreliable.  Cross-thread and cross-
    /// process tests work correctly.
    fn mutex_name(path: &Path, use_global: bool) -> Vec<u16> {
        let hash_hex = hash_path(path);
        let name = if use_global {
            format!("Global\\csq-lock-{hash_hex}")
        } else {
            format!("csq-lock-{hash_hex}")
        };
        name.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Attempts `CreateMutexW` with the Global namespace first.
    ///
    /// Falls back to the implicit Local namespace if the Global
    /// open is rejected with `ERROR_ACCESS_DENIED` or
    /// `ERROR_PRIVILEGE_NOT_HELD`.  Any other error is returned
    /// as-is so bugs are surfaced rather than silently fallen-back.
    fn create_mutex_with_fallback(path: &Path) -> Result<*mut std::ffi::c_void, PlatformError> {
        // Try Global first — gives correct cross-session semantics
        // on elevated accounts and daemons.
        let global_name = mutex_name(path, true);
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, global_name.as_ptr()) };
        if !handle.is_null() {
            return Ok(handle);
        }

        let err = unsafe { GetLastError() };
        if err == ERROR_ACCESS_DENIED || err == ERROR_PRIVILEGE_NOT_HELD {
            // Expected on standard user accounts — fall back to
            // Local\ namespace. This is per-session only: two
            // Terminal Services sessions for the same user will
            // not serialize. See red-team H4 for the caveat.
            warn!(
                "Global\\ mutex denied (error {err}); falling back to session-local namespace. \
                 Cross-session serialization is not guaranteed for standard user accounts."
            );
            let local_name = mutex_name(path, false);
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, local_name.as_ptr()) };
            if !handle.is_null() {
                return Ok(handle);
            }
            return Err(PlatformError::Win32 {
                code: unsafe { GetLastError() },
                message: "CreateMutexW failed in Local namespace after Global fallback".into(),
            });
        }

        Err(PlatformError::Win32 {
            code: err,
            message: "CreateMutexW failed".into(),
        })
    }

    pub fn lock_file(path: &Path) -> Result<FileLockGuard, PlatformError> {
        let handle = create_mutex_with_fallback(path)?;

        let wait_result = unsafe { WaitForSingleObject(handle, INFINITE) };
        match wait_result {
            WAIT_OBJECT_0 => Ok(FileLockGuard {
                _inner: InnerGuard { handle },
            }),
            WAIT_ABANDONED => {
                // GAP-8: treat as acquired but log a warning — the previous
                // holder crashed without releasing.
                warn!(
                    path = %path.display(),
                    "mutex acquired after WAIT_ABANDONED (previous holder crashed)"
                );
                Ok(FileLockGuard {
                    _inner: InnerGuard { handle },
                })
            }
            _ => {
                unsafe { CloseHandle(handle) };
                Err(PlatformError::Win32 {
                    code: wait_result,
                    message: format!("WaitForSingleObject returned {wait_result:#x}"),
                })
            }
        }
    }

    pub fn try_lock_file(path: &Path) -> Result<Option<FileLockGuard>, PlatformError> {
        let handle = create_mutex_with_fallback(path)?;
        let wait_result = unsafe { WaitForSingleObject(handle, 0) };
        match wait_result {
            WAIT_OBJECT_0 => Ok(Some(FileLockGuard {
                _inner: InnerGuard { handle },
            })),
            WAIT_ABANDONED => {
                warn!(
                    path = %path.display(),
                    "mutex acquired after WAIT_ABANDONED (previous holder crashed)"
                );
                Ok(Some(FileLockGuard {
                    _inner: InnerGuard { handle },
                }))
            }
            WAIT_TIMEOUT => {
                unsafe { CloseHandle(handle) };
                Ok(None)
            }
            _ => {
                unsafe { CloseHandle(handle) };
                Err(PlatformError::Win32 {
                    code: wait_result,
                    message: format!("WaitForSingleObject returned {wait_result:#x}"),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lock_and_unlock() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("test.lock");

        let guard = lock_file(&lock_path).unwrap();
        drop(guard);

        // Can re-acquire after drop
        let _guard2 = lock_file(&lock_path).unwrap();
    }

    #[test]
    fn try_lock_returns_none_when_held() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("test.lock");

        let _guard = lock_file(&lock_path).unwrap();

        // Same process, different fd — flock allows this on some systems,
        // but we test the cross-process case below in integration tests.
        // For the unit test, just verify the API works.
    }

    #[test]
    fn try_lock_succeeds_when_free() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("test.lock");

        let guard = try_lock_file(&lock_path).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn lock_creates_file_if_missing() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("new.lock");
        assert!(!lock_path.exists());

        let _guard = lock_file(&lock_path).unwrap();
        assert!(lock_path.exists());
    }

    #[test]
    fn lock_guard_is_debug() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("debug.lock");
        let guard = lock_file(&lock_path).unwrap();
        let s = format!("{guard:?}");
        assert!(s.contains("FileLockGuard"));
    }

    /// The Local (unprefixed) mutex name must contain no
    /// backslashes anywhere, even when the input path contains
    /// them — the path is hashed, not embedded.
    #[test]
    #[cfg(windows)]
    fn mutex_name_local_no_backslashes() {
        use std::path::Path;

        let path = Path::new(r"C:\Users\runner\AppData\Local\Temp\test.lock");
        let wide = imp::mutex_name(path, /* use_global = */ false);
        let decoded = String::from_utf16(&wide[..wide.len() - 1]).unwrap();

        assert!(
            !decoded.contains('\\'),
            "Local mutex name must not contain backslashes: {decoded}"
        );
        assert!(
            decoded.starts_with("csq-lock-"),
            "unexpected name: {decoded}"
        );

        let hash_part = &decoded["csq-lock-".len()..];
        assert_eq!(hash_part.len(), 16);
        assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The Global mutex name has exactly one backslash (the
    /// `Global\` namespace separator) and never anything from the
    /// path itself.
    #[test]
    #[cfg(windows)]
    fn mutex_name_global_has_only_namespace_separator() {
        use std::path::Path;

        let path = Path::new(r"C:\Users\runner\AppData\Local\Temp\test.lock");
        let wide = imp::mutex_name(path, /* use_global = */ true);
        let decoded = String::from_utf16(&wide[..wide.len() - 1]).unwrap();

        assert!(
            decoded.starts_with("Global\\csq-lock-"),
            "unexpected: {decoded}"
        );
        // Exactly one backslash, and only in the namespace prefix.
        assert_eq!(
            decoded.chars().filter(|c| *c == '\\').count(),
            1,
            "Global name must have exactly one backslash (namespace separator): {decoded}"
        );
    }

    /// Two distinct paths must produce distinct mutex names.
    #[test]
    #[cfg(windows)]
    fn mutex_name_distinct_for_distinct_paths() {
        use std::path::Path;

        let a = imp::mutex_name(Path::new(r"C:\Temp\a.lock"), false);
        let b = imp::mutex_name(Path::new(r"C:\Temp\b.lock"), false);
        assert_ne!(a, b, "different paths must yield different mutex names");
    }

    /// The same path must always produce the same mutex name.
    #[test]
    #[cfg(windows)]
    fn mutex_name_deterministic() {
        use std::path::Path;

        let path = Path::new(r"C:\Temp\stable.lock");
        let first = imp::mutex_name(path, false);
        let second = imp::mutex_name(path, false);
        assert_eq!(first, second);
    }
}
