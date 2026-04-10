//! `csq login <N>` — OAuth login flow for a new account.
//!
//! Opens `claude auth login` with an isolated CLAUDE_CONFIG_DIR, then
//! captures the resulting credentials from keychain or file.

use anyhow::{anyhow, Context, Result};
use csq_core::accounts::{markers, profiles};
use csq_core::broker::fanout;
use csq_core::credentials::{self, file, keychain};
use csq_core::types::AccountNum;
use std::path::Path;
use std::process::Command;

pub fn handle(base_dir: &Path, account: AccountNum) -> Result<()> {
    let config_dir = base_dir.join(format!("config-{}", account));
    std::fs::create_dir_all(&config_dir)?;

    // Mark this dir with the account number early so recovery is possible
    markers::write_csq_account(&config_dir, account)?;

    println!("Starting OAuth login for account {}...", account);
    println!("Your browser will open for authorization.");

    // Invoke `claude auth login` with isolated config dir
    let status = Command::new("claude")
        .args(["auth", "login"])
        .env("CLAUDE_CONFIG_DIR", &config_dir)
        .status()
        .context("failed to spawn `claude auth login` — is Claude Code installed?")?;

    if !status.success() {
        return Err(anyhow!("claude auth login exited with non-zero status"));
    }

    // Capture credentials — try keychain first, then file
    let captured = keychain::read(&config_dir)
        .or_else(|| credentials::load(&config_dir.join(".credentials.json")).ok());

    let creds = captured.ok_or_else(|| {
        anyhow!("no credentials captured after login — keychain and file both empty")
    })?;

    // Save canonical + mirror
    file::save_canonical(base_dir, account, &creds)?;
    println!(
        "Credentials saved to {}",
        file::canonical_path(base_dir, account).display()
    );

    // Capture email via `claude auth status --json`
    let email = get_email_from_cc(&config_dir).unwrap_or_else(|_| "unknown".to_string());
    update_profile(base_dir, account, &email)?;

    // Clear broker-failed flag
    fanout::clear_broker_failed(base_dir, account);

    println!("Logged in as {} (account {}).", email, account);
    Ok(())
}

fn get_email_from_cc(config_dir: &Path) -> Result<String> {
    let output = Command::new("claude")
        .args(["auth", "status", "--json"])
        .env("CLAUDE_CONFIG_DIR", config_dir)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("claude auth status failed"));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    json.get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("no email in claude auth status output"))
}

fn update_profile(base_dir: &Path, account: AccountNum, email: &str) -> Result<()> {
    let path = profiles::profiles_path(base_dir);
    let mut profiles = profiles::load(&path).unwrap_or_else(|_| profiles::ProfilesFile::empty());

    profiles.set_profile(
        account.get(),
        profiles::AccountProfile {
            email: email.to_string(),
            method: "oauth".to_string(),
            extra: std::collections::HashMap::new(),
        },
    );

    profiles::save(&path, &profiles)?;
    Ok(())
}
