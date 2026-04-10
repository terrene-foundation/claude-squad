use csq_core::accounts::discovery;
use csq_core::accounts::AccountSource;
use csq_core::broker::fanout;
use csq_core::quota::state as quota_state;
use csq_core::quota::QuotaFile;
use csq_core::rotation;
use csq_core::rotation::config as rotation_config;
use csq_core::rotation::RotationConfig;
use csq_core::types::AccountNum;
use serde::Serialize;
use std::path::PathBuf;

/// Public view of a single account, safe to send over IPC.
///
/// Credentials, tokens, and keys are never included.
#[derive(Serialize)]
pub struct AccountView {
    pub id: u16,
    pub label: String,
    /// "anthropic" | "third_party" | "manual"
    pub source: String,
    pub has_credentials: bool,
    pub five_hour_pct: f64,
    pub seven_day_pct: f64,
    pub updated_at: f64,
}

/// Returns all configured accounts with current quota data.
///
/// `base_dir` is the Claude accounts directory (e.g. `~/.claude/accounts`).
/// Returns a validation error if the directory does not exist.
#[tauri::command]
pub fn get_accounts(base_dir: String) -> Result<Vec<AccountView>, String> {
    let base = PathBuf::from(&base_dir);
    if !base.is_dir() {
        return Err(format!(
            "base directory does not exist: {base_dir}"
        ));
    }

    let accounts = discovery::discover_all(&base);
    let quota: QuotaFile = quota_state::load_state(&base).unwrap_or_else(|_| QuotaFile::empty());

    let views = accounts
        .into_iter()
        .map(|a| {
            let q = quota.get(a.id);
            AccountView {
                id: a.id,
                label: a.label,
                source: match a.source {
                    AccountSource::Anthropic => "anthropic".into(),
                    AccountSource::ThirdParty { .. } => "third_party".into(),
                    AccountSource::Manual => "manual".into(),
                },
                has_credentials: a.has_credentials,
                five_hour_pct: q.map(|q| q.five_hour_pct()).unwrap_or(0.0),
                seven_day_pct: q.map(|q| q.seven_day_pct()).unwrap_or(0.0),
                updated_at: q.map(|q| q.updated_at).unwrap_or(0.0),
            }
        })
        .collect();

    Ok(views)
}

/// Swaps the active account in the first config dir found for `target`.
///
/// `base_dir` is the Claude accounts directory. `target` must be 1–999.
/// Returns an error if no active session exists for the account.
#[tauri::command]
pub fn swap_account(base_dir: String, target: u16) -> Result<String, String> {
    let base = PathBuf::from(&base_dir);

    let account = AccountNum::try_from(target)
        .map_err(|e| format!("invalid account: {e}"))?;

    let config_dirs = fanout::scan_config_dirs(&base, account);
    let config_dir = config_dirs
        .first()
        .ok_or_else(|| format!("no active session for account {target}"))?;

    rotation::swap_to(&base, config_dir, account)
        .map(|r| format!("Swapped to account {}", r.account))
        .map_err(|e| e.to_string())
}

/// Returns the current auto-rotation configuration.
///
/// Returns defaults if `rotation.json` does not exist.
#[tauri::command]
pub fn get_rotation_config(base_dir: String) -> Result<RotationConfig, String> {
    let base = PathBuf::from(&base_dir);
    rotation_config::load(&base).map_err(|e| e.to_string())
}

/// Enables or disables auto-rotation, writing the change to `rotation.json`.
#[tauri::command]
pub fn set_rotation_enabled(base_dir: String, enabled: bool) -> Result<(), String> {
    let base = PathBuf::from(&base_dir);
    let mut config = rotation_config::load(&base).map_err(|e| e.to_string())?;
    config.enabled = enabled;
    rotation_config::save(&base, &config).map_err(|e| e.to_string())
}
