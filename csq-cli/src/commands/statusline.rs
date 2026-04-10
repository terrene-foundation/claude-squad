//! `csq statusline` — reads CC JSON from stdin, outputs formatted statusline.

use anyhow::Result;
use csq_core::accounts::markers;
use csq_core::broker::fanout::is_broker_failed;
use csq_core::quota::{
    format::{account_label, is_swap_stuck, should_report_broker_failed, statusline_str},
    state,
};
use csq_core::types::AccountNum;
use std::io::Read;
use std::path::Path;

pub fn handle(base_dir: &Path) -> Result<()> {
    let config_dir = super::current_config_dir();

    // Read CC's JSON from stdin (may be empty — statusline runs even without CC payload)
    let mut stdin_json = String::new();
    let _ = std::io::stdin().read_to_string(&mut stdin_json);

    // Determine active account
    let account: AccountNum = match config_dir.as_deref().and_then(markers::read_current_account) {
        Some(a) => a,
        None => {
            // Fall back to any first account
            println!("csq: no active account");
            return Ok(());
        }
    };

    let config_dir = config_dir.unwrap();
    let quota = state::load_state(base_dir).unwrap_or_else(|_| csq_core::quota::QuotaFile::empty());
    let account_quota = quota.get(account.get());

    let label = account_label(base_dir, account);
    let stuck = is_swap_stuck(&config_dir, base_dir);
    let broker_failed = should_report_broker_failed(base_dir, account)
        || is_broker_failed(base_dir, account);

    let line = statusline_str(account, &label, account_quota, stuck, broker_failed);
    println!("{line}");
    Ok(())
}
