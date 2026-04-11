mod commands;

use csq_core::accounts::discovery;
use csq_core::rotation;
use csq_core::types::AccountNum;
use std::path::{Path, PathBuf};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager};

/// Maximum length of an account label shown in the tray.
const MAX_LABEL_LEN: usize = 64;

/// Returns the base directory for csq state — `~/.claude/accounts`.
///
/// Honors the `CSQ_BASE_DIR` environment variable for testing.
fn base_dir() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("CSQ_BASE_DIR") {
        return Some(PathBuf::from(override_path));
    }
    let home = dirs::home_dir()?;
    Some(home.join(".claude").join("accounts"))
}

/// Sanitizes a label for display in the tray menu.
///
/// Strips control characters and Unicode bidirectional overrides
/// (homograph attack vector) and caps length. Labels come from
/// `profiles.json`, which is user-writable — a misbehaving tool
/// could inject newlines, ANSI-like sequences, or RTL overrides
/// that mangle the menu rendering.
fn sanitize_label(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| {
            !c.is_control()
                // Bidirectional overrides: LRO, RLO, LRE, RLE, PDF, LRI, RLI, FSI, PDI
                && !matches!(
                    *c,
                    '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
                )
        })
        .take(MAX_LABEL_LEN)
        .collect();
    if cleaned.is_empty() {
        "unknown".into()
    } else {
        cleaned
    }
}

/// Returns the config-N directory whose `.credentials.json` was
/// most recently modified, where `N` is a valid account number
/// (1..=999).
///
/// **Why `.credentials.json` mtime, not dir mtime**: directory
/// mtime changes only on child entry creation/deletion/rename. A
/// user actively using `config-5` (reading the file, modifying
/// in-place state) would NOT bump the dir's mtime, so a dir-mtime
/// heuristic picks whichever dir had a credential rotated last in
/// an auto-refresh sweep — not the dir the user's terminal is on.
/// The `.credentials.json` file itself is rewritten via
/// `atomic_replace` on every token refresh and login, and those
/// writes happen against the specific active dir.
///
/// Tray quick-swap targets only ONE config dir — retargeting every
/// live session at once is destructive and silent. Picking the
/// most-recently-rewritten credential approximates "the session
/// the user is actively using" without needing `CLAUDE_CONFIG_DIR`
/// (which GUI-launched apps don't inherit).
///
/// Rejects:
/// * Non-directories
/// * Symlinks (could redirect writes outside base_dir)
/// * Names not matching `config-<1..=999>`
/// * Directories with no readable `.credentials.json`
fn most_recent_config_dir(base: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;

    for entry in entries.flatten() {
        // Reject symlinks via `file_type()` which does NOT follow.
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() || !ft.is_dir() {
            continue;
        }

        let name_os = entry.file_name();
        let Some(name) = name_os.to_str() else {
            continue;
        };

        // Must match `config-<1..=999>` exactly.
        let Some(num_str) = name.strip_prefix("config-") else {
            continue;
        };
        let Ok(n) = num_str.parse::<u16>() else {
            continue;
        };
        if !(1..=999).contains(&n) {
            continue;
        }

        // Signal: mtime of `{dir}/.credentials.json`, which is
        // rewritten via atomic_replace on every refresh/login.
        // Dirs without a credentials file are skipped entirely —
        // they're not live sessions.
        let path = entry.path();
        let cred_path = path.join(".credentials.json");
        let Ok(meta) = std::fs::metadata(&cred_path) else {
            continue;
        };
        let Ok(mtime) = meta.modified() else { continue };

        match best.as_ref() {
            None => best = Some((mtime, path)),
            Some((t, _)) if mtime > *t => best = Some((mtime, path)),
            _ => {}
        }
    }

    best.map(|(_, path)| path)
}

/// Builds the tray menu from the current account list.
///
/// Menu layout:
///   #{id} {label}  ← one row per account (no active checkmark —
///                    see note below)
///   ---
///   Open Dashboard
///   Hide Dashboard
///   ---
///   Quit Claude Squad
///
/// No checkmark is shown for an "active" account because the
/// desktop app has no single active session — each live config-*
/// dir has its own active account, and `CLAUDE_CONFIG_DIR` is not
/// set in a GUI-launched Tauri process, so there is no unambiguous
/// signal to choose. The tray action is a "quick-swap" that
/// retargets *all* live config dirs to the clicked account.
fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let mut builder = MenuBuilder::new(app);

    if let Some(base) = base_dir() {
        if base.is_dir() {
            let accounts = discovery::discover_anthropic(&base);
            let mut had_any = false;
            for a in &accounts {
                if !a.has_credentials {
                    continue;
                }
                had_any = true;
                let label = format!("#{} {}", a.id, sanitize_label(&a.label));
                let id = format!("acct:{}", a.id);
                let item = MenuItemBuilder::with_id(id, label).build(app)?;
                builder = builder.item(&item);
            }
            if had_any {
                builder = builder.item(&PredefinedMenuItem::separator(app)?);
            }
        }
    }

    let open_dashboard = MenuItemBuilder::with_id("open", "Open Dashboard").build(app)?;
    let hide_dashboard = MenuItemBuilder::with_id("hide", "Hide Dashboard").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Claude Squad").build(app)?;

    builder
        .item(&open_dashboard)
        .item(&hide_dashboard)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit)
        .build()
}

/// Outcome of a tray swap click emitted as `tray-swap-complete`.
#[derive(serde::Serialize, Clone)]
struct TraySwapResult {
    account: u16,
    /// The config dir that was retargeted, if any.
    config_dir: Option<String>,
    /// `true` on success; `false` if no dir found or swap failed.
    ok: bool,
    /// Human-readable error if `ok == false`.
    error: Option<String>,
}

/// Performs the blocking swap work for a tray `acct:{id}` click.
///
/// Retargets the **single most recently modified** `config-N` dir
/// to the clicked account. Running on a tokio `spawn_blocking`
/// worker — MUST not be invoked from the Tauri main thread.
///
/// # Why one dir, not all
///
/// Retargeting every live `config-*` dir would silently collapse a
/// multi-session workflow (5 CC sessions on 5 accounts → all on
/// one account) with no confirmation. The most-recently-modified
/// dir approximates "the session the user is actively using" and
/// matches the intent of a tray quick-switch.
fn perform_tray_swap(base: &Path, account: AccountNum) -> TraySwapResult {
    let target_dir = match most_recent_config_dir(base) {
        Some(d) => d,
        None => {
            log::warn!(
                "tray swap: no live config-N dir under {} — start a CC session first",
                base.display()
            );
            return TraySwapResult {
                account: account.get(),
                config_dir: None,
                ok: false,
                error: Some("no live CC session found".into()),
            };
        }
    };

    match rotation::swap_to(base, &target_dir, account) {
        Ok(res) => {
            log::info!(
                "tray swap ok: account {} -> {}",
                res.account,
                target_dir.display()
            );
            TraySwapResult {
                account: account.get(),
                config_dir: Some(target_dir.display().to_string()),
                ok: true,
                error: None,
            }
        }
        Err(e) => {
            log::warn!(
                "tray swap failed: account {} -> {}: {}",
                account,
                target_dir.display(),
                e
            );
            TraySwapResult {
                account: account.get(),
                config_dir: Some(target_dir.display().to_string()),
                ok: false,
                error: Some(e.to_string()),
            }
        }
    }
}

/// Serialization guard for tray clicks — atomic "is a swap in
/// flight?" flag. Prevents concurrent tray clicks from racing each
/// other's writes to the same config dir. A click arriving while
/// another swap is in-flight is dropped with a log line (not
/// queued — queuing leads to confusing "which click won?" UX).
static SWAP_IN_FLIGHT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Handles a tray menu click.
///
/// Account rows dispatch the swap work to a `spawn_blocking` worker
/// so the Tauri main thread (UI, tray rendering) stays responsive.
/// Concurrent clicks are serialized via `SWAP_IN_FLIGHT` — a
/// subsequent click while a swap is running is dropped to avoid
/// non-deterministic writes.
fn handle_tray_event(app: &AppHandle, id: &str) {
    match id {
        "open" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "hide" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
        }
        "quit" => {
            app.exit(0);
        }
        s if s.starts_with("acct:") => {
            let Some(num_str) = s.strip_prefix("acct:") else {
                return;
            };
            let Ok(n) = num_str.parse::<u16>() else {
                return;
            };
            let Ok(account) = AccountNum::try_from(n) else {
                return;
            };
            let Some(base) = base_dir() else { return };

            // Serialize: drop the click if another swap is in
            // flight. Release-ordered CAS so only one worker runs.
            use std::sync::atomic::Ordering;
            if SWAP_IN_FLIGHT
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                log::warn!(
                    "tray click ignored: account {} — another swap is in flight",
                    n
                );
                return;
            }

            let app_handle = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                // RAII guard so the flag always clears, even on
                // panic inside perform_tray_swap.
                struct ClearFlag;
                impl Drop for ClearFlag {
                    fn drop(&mut self) {
                        SWAP_IN_FLIGHT.store(false, Ordering::Release);
                    }
                }
                let _clear = ClearFlag;

                let result = perform_tray_swap(&base, account);
                if let Err(e) = app_handle.emit("tray-swap-complete", &result) {
                    log::warn!("failed to emit tray-swap-complete: {e}");
                }
            });
        }
        _ => {}
    }
}

/// Rebuilds and reattaches the tray menu.
///
/// Called on a 30s interval so the tray reflects account additions,
/// deletions, and active-session changes made from the CLI or other
/// processes.
fn refresh_tray_menu(app: &AppHandle, tray: &TrayIcon) {
    if let Ok(menu) = build_tray_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_accounts,
            commands::swap_account,
            commands::get_rotation_config,
            commands::set_rotation_enabled,
            commands::get_daemon_status,
            commands::start_login,
        ])
        .setup(|app| {
            // ── Logging ────────────────────────────────────────
            //
            // Two independent logging facades coexist:
            //
            // 1. **tracing** — csq-core emits via `tracing::warn!`
            //    etc. A `tracing_subscriber::fmt` subscriber
            //    writes those events to stderr filtered by
            //    `CSQ_LOG` (default: `warn`).
            //
            // 2. **log** — `tauri-plugin-log` claims the `log`
            //    facade for tray-click errors and plugin
            //    lifecycle messages. Output goes to the OS app-
            //    data log dir (Console.app on macOS, etc.).
            //
            // **Critical**: `tracing-subscriber`'s default
            // features include `tracing-log`, which would make
            // `try_init()` silently call `log::set_logger`. That
            // then collides with `tauri-plugin-log`'s own
            // `set_boxed_logger` and panics the app at startup.
            // The workspace `tracing-subscriber` dep is
            // configured with `default-features = false` +
            // explicit `fmt`/`env-filter`/`std`/`ansi`/`smallvec`
            // to avoid this collision. See Cargo.toml.
            let filter = tracing_subscriber::EnvFilter::try_from_env("CSQ_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::io::stderr)
                .try_init();

            let log_level = if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            };
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log_level)
                    .build(),
            )?;

            // ── Auto-updater ─────────────────────────────────
            // Registers the updater plugin. Actual update checks require
            // a signed update manifest at the configured endpoint.
            // Signing keys and update server are configured in M11.
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // ── System tray ──────────────────────────────────
            let menu = build_tray_menu(app.handle())?;
            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Claude Squad")
                .on_menu_event(move |app, event| {
                    handle_tray_event(app, event.id().as_ref());
                })
                .build(app)?;

            // Refresh the tray menu every 30s so account changes
            // made from the CLI show up without restarting the app.
            //
            // `MissedTickBehavior::Skip` prevents the ticker from
            // firing N catch-up ticks when the process wakes from
            // laptop sleep — we only ever want the next scheduled
            // tick after a gap, not a burst of 20 catch-ups.
            let app_handle = app.handle().clone();
            let tray_handle = tray.clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                // First tick fires immediately; skip it since we
                // just built the menu synchronously above.
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    refresh_tray_menu(&app_handle, &tray_handle);
                }
            });

            // Hide window on close instead of quitting (tray keeps app alive)
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    // ── sanitize_label ──────────────────────────────────────

    #[test]
    fn sanitize_label_strips_control_chars() {
        assert_eq!(sanitize_label("alice\nbob"), "alicebob");
        assert_eq!(sanitize_label("a\tb\rc"), "abc");
        assert_eq!(sanitize_label("x\u{0}y"), "xy");
    }

    #[test]
    fn sanitize_label_strips_bidi_overrides() {
        // U+202E is Right-to-Left Override (homograph attack)
        assert_eq!(
            sanitize_label("gro.eniRrreT\u{202E}@alice"),
            "gro.eniRrreT@alice"
        );
        // U+2066..=U+2069 are isolates
        assert_eq!(sanitize_label("a\u{2066}b\u{2069}c"), "abc");
        // U+202A..=U+202D are other bidi controls
        assert_eq!(sanitize_label("a\u{202A}b\u{202B}c\u{202C}d"), "abcd");
    }

    #[test]
    fn sanitize_label_caps_length() {
        let long = "x".repeat(200);
        let out = sanitize_label(&long);
        assert_eq!(out.chars().count(), MAX_LABEL_LEN);
    }

    #[test]
    fn sanitize_label_empty_returns_placeholder() {
        assert_eq!(sanitize_label(""), "unknown");
        // Also when everything gets stripped.
        assert_eq!(sanitize_label("\n\r\t"), "unknown");
    }

    #[test]
    fn sanitize_label_preserves_normal_unicode() {
        assert_eq!(sanitize_label("alice@example.com"), "alice@example.com");
        // Non-ASCII but not a control/bidi char.
        assert_eq!(sanitize_label("Ålice"), "Ålice");
    }

    // ── most_recent_config_dir ──────────────────────────────

    fn touch_credentials(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(".credentials.json"), b"{}").unwrap();
    }

    #[test]
    fn most_recent_picks_newest_credentials_mtime() {
        let base = TempDir::new().unwrap();
        touch_credentials(&base.path().join("config-1"));
        thread::sleep(Duration::from_millis(20));
        touch_credentials(&base.path().join("config-2"));
        thread::sleep(Duration::from_millis(20));
        touch_credentials(&base.path().join("config-3"));

        let result = most_recent_config_dir(base.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "config-3");
    }

    #[test]
    fn most_recent_skips_dirs_without_credentials() {
        let base = TempDir::new().unwrap();
        fs::create_dir_all(base.path().join("config-1")).unwrap();
        touch_credentials(&base.path().join("config-2"));

        let result = most_recent_config_dir(base.path()).unwrap();
        assert_eq!(result.file_name().unwrap(), "config-2");
    }

    #[test]
    fn most_recent_rejects_out_of_range_numbers() {
        let base = TempDir::new().unwrap();
        touch_credentials(&base.path().join("config-0"));
        touch_credentials(&base.path().join("config-1000"));

        assert!(most_recent_config_dir(base.path()).is_none());
    }

    #[test]
    fn most_recent_rejects_non_numeric_suffix() {
        let base = TempDir::new().unwrap();
        touch_credentials(&base.path().join("config-abc"));
        touch_credentials(&base.path().join("config-"));

        assert!(most_recent_config_dir(base.path()).is_none());
    }

    #[test]
    fn most_recent_rejects_non_config_prefix() {
        let base = TempDir::new().unwrap();
        touch_credentials(&base.path().join("other-1"));
        touch_credentials(&base.path().join("xconfig-1"));

        assert!(most_recent_config_dir(base.path()).is_none());
    }

    #[test]
    fn most_recent_returns_none_on_empty_dir() {
        let base = TempDir::new().unwrap();
        assert!(most_recent_config_dir(base.path()).is_none());
    }

    #[test]
    fn most_recent_returns_none_when_base_missing() {
        let base = TempDir::new().unwrap();
        let missing = base.path().join("missing-dir");
        assert!(most_recent_config_dir(&missing).is_none());
    }

    #[test]
    #[cfg(unix)]
    fn most_recent_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let base = TempDir::new().unwrap();
        // Real dir outside base that we want to protect.
        let outside = TempDir::new().unwrap();
        touch_credentials(outside.path());

        // Create a symlink config-5 → outside. file_type() must NOT
        // follow the symlink, so this must be rejected.
        symlink(outside.path(), base.path().join("config-5")).unwrap();

        assert!(
            most_recent_config_dir(base.path()).is_none(),
            "symlinks must be rejected to prevent writes outside base_dir"
        );
    }
}
