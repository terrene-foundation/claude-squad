//! `csq run [N]` — launch Claude Code with isolated credentials.

use anyhow::{anyhow, Context, Result};
use csq_core::accounts::{discovery, markers};
use csq_core::broker;
use csq_core::session;
use csq_core::types::AccountNum;
use std::path::Path;
use std::process::Command;

pub fn handle(
    base_dir: &Path,
    account: Option<AccountNum>,
    profile: Option<&str>,
    rest: &[String],
) -> Result<()> {
    let claude_home = super::claude_home()?;

    // Resolve account number
    let account = resolve_account(base_dir, account)?;

    let account = match account {
        Some(a) => a,
        None => {
            // 0 accounts — launch vanilla claude
            println!("No accounts configured — launching vanilla claude.");
            return exec_claude(rest);
        }
    };

    // Set up config dir
    let config_dir = base_dir.join(format!("config-{}", account));
    std::fs::create_dir_all(&config_dir)?;

    // Isolate: symlink shared items
    session::isolate_config_dir(&claude_home, &config_dir)
        .context("failed to isolate config dir")?;

    // Mark account
    markers::write_csq_account(&config_dir, account)?;
    markers::write_current_account(&config_dir, account)?;

    // Cleanup stale PID
    session::setup::cleanup_stale_pid(&config_dir);

    // Mark onboarding complete
    session::mark_onboarding_complete(&config_dir)?;

    // Synchronous broker check — refresh token if needed
    let refresh_fn = |url: &str, body: &str| -> Result<Vec<u8>, String> {
        // Use ureq for a simple synchronous HTTP POST
        // For now, return a stub error — a real HTTP client will be added later
        Err(format!("HTTP client not yet wired ({url}): {}", body.len()))
    };
    let _ = broker::check::broker_check(base_dir, account, refresh_fn);

    // Copy credentials for the session
    session::setup::copy_credentials_for_session(base_dir, &config_dir, account)
        .context("failed to copy credentials")?;

    // Merge profile settings if specified
    if let Some(profile_id) = profile {
        println!("Using profile: {profile_id}");
        // TODO: merge profile settings into config_dir/settings.json
    }

    println!("Launching claude for account {}...", account);

    // Strip ANTHROPIC_* env vars before exec
    let mut cmd = Command::new("claude");
    cmd.env("CLAUDE_CONFIG_DIR", &config_dir);
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.env_remove("ANTHROPIC_AUTH_TOKEN");
    cmd.args(rest);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        Err(anyhow!("exec failed: {err}"))
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

fn resolve_account(
    base_dir: &Path,
    explicit: Option<AccountNum>,
) -> Result<Option<AccountNum>> {
    if let Some(a) = explicit {
        return Ok(Some(a));
    }

    let accounts = discovery::discover_anthropic(base_dir);
    let anthropic_with_creds: Vec<_> = accounts.iter().filter(|a| a.has_credentials).collect();

    match anthropic_with_creds.len() {
        0 => Ok(None), // vanilla claude
        1 => {
            let num = AccountNum::try_from(anthropic_with_creds[0].id)
                .map_err(|e| anyhow!("invalid account: {e}"))?;
            Ok(Some(num))
        }
        _ => {
            let mut msg = String::from("multiple accounts configured — specify one:\n");
            for a in &anthropic_with_creds {
                msg.push_str(&format!("  csq run {}  ({})\n", a.id, a.label));
            }
            Err(anyhow!(msg))
        }
    }
}

fn exec_claude(rest: &[String]) -> Result<()> {
    let mut cmd = Command::new("claude");
    cmd.args(rest);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        Err(anyhow!("exec failed: {err}"))
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
