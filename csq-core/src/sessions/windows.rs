//! Windows backend for live CC session discovery — currently a stub.
//!
//! Reading another process's environment on Windows requires
//! `NtQueryInformationProcess(ProcessBasicInformation)` to get a
//! `PEB` pointer, then `ReadProcessMemory` to walk the
//! `RTL_USER_PROCESS_PARAMETERS.Environment` block. That needs
//! unsafe code and version gating across Windows 10/11 PEB layout
//! changes, and is too much to get right in an autonomous session
//! without being able to run it against real Windows targets.
//!
//! Until a later session stands up a proper backend, `list` returns
//! an empty vector. The Tauri command's error path already handles
//! an empty list gracefully, so Windows users see a "No live CC
//! sessions found" empty state — same as a fresh macOS/Linux
//! install with no terminals running.

use super::SessionInfo;

pub fn list() -> Vec<SessionInfo> {
    Vec::new()
}
