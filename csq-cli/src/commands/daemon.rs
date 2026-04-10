//! `csq daemon start/stop/status` — background daemon lifecycle.
//!
//! # M8.2 scope
//!
//! This slice implements the foreground-only daemon lifecycle. The
//! daemon runs in the current terminal, writes its PID file, installs
//! SIGTERM/SIGINT handlers, and blocks until a shutdown signal
//! arrives. There is no background/detached mode yet — that will
//! arrive in M8.6 when the daemon is hosted inside the Tauri tray
//! app (see the roadmap in `workspaces/csq-v2/todos/active/`).
//!
//! There is also no IPC server yet (M8.3), no background refresher
//! (M8.4), no HTTP API routes (M8.5). The daemon currently does
//! *nothing* except hold the PID file open. This is intentional: it
//! gives us a clean, testable lifecycle primitive to build the
//! subsystems on top of.

use anyhow::{Context, Result};
use csq_core::daemon::{self, DaemonStatus, PidFile};
use std::path::Path;

/// Runs `csq daemon start` in the foreground.
///
/// Acquires the PID file (failing if another daemon is already
/// running), creates a minimal tokio runtime, installs signal
/// handlers, and blocks until SIGTERM/SIGINT. On return, the PID
/// file is automatically removed via `PidFile`'s Drop impl.
pub fn handle_start(base_dir: &Path) -> Result<()> {
    let pid_path = daemon::pid_file_path(base_dir);

    // Acquire PID file; errors if another daemon is already running.
    let pid_file = PidFile::acquire(&pid_path).with_context(|| {
        format!("could not acquire PID file at {}", pid_path.display())
    })?;

    eprintln!(
        "csq daemon started (PID {}, foreground mode)",
        pid_file.owned_pid()
    );
    eprintln!("  PID file: {}", pid_file.path().display());
    eprintln!("  Socket:   {} (not active — M8.3)", daemon::socket_path(base_dir).display());
    eprintln!("Send SIGTERM (kill {}) or Ctrl-C to stop.", pid_file.owned_pid());

    // Build a small current-thread tokio runtime for the signal
    // wait. M8.3 will upgrade this to a multi-threaded runtime when
    // the IPC server and background tasks arrive.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for daemon")?;

    rt.block_on(wait_for_shutdown());

    eprintln!("csq daemon stopping...");
    // Explicit drop for clarity — PidFile::Drop removes the file if
    // it still contains our PID.
    drop(pid_file);
    eprintln!("csq daemon stopped cleanly");

    Ok(())
}

/// Runs `csq daemon stop` — sends SIGTERM to the running daemon and
/// polls for exit.
pub fn handle_stop(base_dir: &Path) -> Result<()> {
    let pid_path = daemon::pid_file_path(base_dir);

    match daemon::stop_daemon(&pid_path) {
        Ok(pid) => {
            eprintln!("csq daemon stopped (PID {pid})");
            Ok(())
        }
        Err(csq_core::error::DaemonError::NotRunning { .. }) => {
            eprintln!("csq daemon not running");
            Ok(())
        }
        Err(csq_core::error::DaemonError::StalePidFile { pid }) => {
            eprintln!("csq daemon stale PID file (PID {pid} not alive) — cleaned up");
            Ok(())
        }
        Err(csq_core::error::DaemonError::IpcTimeout { timeout_ms }) => {
            anyhow::bail!(
                "csq daemon did not exit within {timeout_ms}ms of SIGTERM \
                 — process may be stuck; investigate before sending SIGKILL"
            )
        }
        Err(e) => Err(e.into()),
    }
}

/// Runs `csq daemon status` — reports running/stale/stopped.
///
/// Returns Ok(()) in all cases so `csq daemon status` never fails
/// for informational queries. Exit code reflects status for shell
/// scripting: 0 = running, 1 = stopped/stale.
pub fn handle_status(base_dir: &Path) -> Result<()> {
    let pid_path = daemon::pid_file_path(base_dir);

    match daemon::status_of(&pid_path) {
        DaemonStatus::Running { pid } => {
            println!("running");
            eprintln!("  PID:      {pid}");
            eprintln!("  PID file: {}", pid_path.display());
            eprintln!("  Socket:   {} (M8.3)", daemon::socket_path(base_dir).display());
            Ok(())
        }
        DaemonStatus::Stale { pid } => {
            println!("stale");
            eprintln!("  PID file references dead PID {pid} at {}", pid_path.display());
            eprintln!("  Run `csq daemon start` to clean up and restart.");
            std::process::exit(1);
        }
        DaemonStatus::NotRunning => {
            println!("not running");
            std::process::exit(1);
        }
    }
}

/// Waits for SIGTERM or SIGINT (Unix) / Ctrl-C (Windows).
///
/// Returns as soon as either signal arrives. Must be called from
/// within a tokio runtime context.
async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        let mut int = signal(SignalKind::interrupt())
            .expect("failed to install SIGINT handler");
        tokio::select! {
            _ = term.recv() => tracing::info!("SIGTERM received"),
            _ = int.recv() => tracing::info!("SIGINT received"),
        }
    }
    #[cfg(windows)]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
        tracing::info!("Ctrl-C received");
    }
}
