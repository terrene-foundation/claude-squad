//! `csq status` — display all accounts with quota usage.

use anyhow::Result;
use csq_core::accounts::markers;
use csq_core::quota::status::show_status;
use std::path::Path;

pub fn handle(base_dir: &Path) -> Result<()> {
    // Try to resolve active account from current config dir marker
    let active = super::current_config_dir()
        .as_deref()
        .and_then(markers::read_current_account);

    let accounts = show_status(base_dir, active);

    if accounts.is_empty() {
        println!("No accounts configured.");
        println!();
        println!("Run `csq login 1` to add your first account.");
        return Ok(());
    }

    println!();
    for account in &accounts {
        println!("{}", account.format_line());
    }
    println!();

    Ok(())
}
