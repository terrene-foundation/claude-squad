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

/// Returns every `config-*` directory under base_dir.
///
/// Unlike `fanout::scan_config_dirs` which filters by the marker
/// account, this returns ALL live config dirs regardless of which
/// account they currently hold. Used by the tray quick-swap to
/// retarget every live session to a single account.
fn list_all_config_dirs(base: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Ok(entries) = std::fs::read_dir(base) else {
        return dirs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("config-") {
            dirs.push(path);
        }
    }
    dirs
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

/// Performs the blocking swap work for a tray `acct:{id}` click.
///
/// Enumerates every live config-* dir and retargets each to the
/// clicked account. Runs on a tokio `spawn_blocking` worker — must
/// not be invoked from the Tauri main thread.
fn perform_tray_swap(base: &Path, account: AccountNum) -> (usize, usize) {
    let config_dirs = list_all_config_dirs(base);
    let mut ok = 0usize;
    let mut err = 0usize;
    for cd in &config_dirs {
        match rotation::swap_to(base, cd, account) {
            Ok(res) => {
                log::info!("tray swap ok: account {} -> {}", res.account, cd.display());
                ok += 1;
            }
            Err(e) => {
                log::warn!(
                    "tray swap failed: account {} -> {}: {}",
                    account,
                    cd.display(),
                    e
                );
                err += 1;
            }
        }
    }
    (ok, err)
}

/// Handles a tray menu click.
///
/// Account rows dispatch the swap work to a `spawn_blocking` worker
/// so the Tauri main thread (UI, tray rendering) stays responsive.
/// Also refreshes the menu immediately after the swap completes so
/// any label changes show without waiting for the next 30s tick.
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

            let app_handle = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let (ok, err) = perform_tray_swap(&base, account);
                // Notify the frontend — a listener in the dashboard
                // can show a toast / refresh data.
                let _ = app_handle.emit(
                    "tray-swap-complete",
                    serde_json::json!({
                        "account": account.get(),
                        "ok": ok,
                        "err": err,
                    }),
                );
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
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

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
