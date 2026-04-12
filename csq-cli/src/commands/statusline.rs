//! `csq statusline` — reads CC JSON from stdin, runs snapshot + sync,
//! and outputs the formatted statusline.
//!
//! ## Account/Terminal Separation
//!
//! This command is a TERMINAL operation. It reads and displays account
//! quota data but NEVER writes it. Quota data is written exclusively
//! by the daemon's usage poller, which polls Anthropic's `/api/oauth/usage`
//! endpoint directly per account.
//!
//! See `rules/account-terminal-separation.md` for the full spec.

use anyhow::Result;
use csq_core::accounts::{markers, snapshot};
use csq_core::broker::{fanout::is_broker_failed, sync};
use csq_core::quota::format::{
    account_label, is_swap_stuck, should_report_broker_failed, statusline_str,
};
use csq_core::quota::state;
use csq_core::types::AccountNum;
use std::io::Read;
use std::path::Path;

/// Maximum bytes of CC JSON we accept on stdin.
/// Real CC payloads are <16KB; 64KB is generous and prevents DoS.
const MAX_STDIN: u64 = 65_536;

pub fn handle(base_dir: &Path) -> Result<()> {
    let config_dir = match super::current_config_dir() {
        Some(d) => d,
        None => {
            println!("csq: no config dir");
            return Ok(());
        }
    };

    // Drain stdin so CC doesn't get a broken pipe.
    // We no longer use the JSON payload for quota updates —
    // that's the daemon's job via Anthropic's usage API.
    let mut sink = String::new();
    let _ = std::io::stdin().take(MAX_STDIN).read_to_string(&mut sink);
    drop(sink);

    // ── Snapshot: identify which account CC is running ──
    let _ = snapshot::snapshot_account(&config_dir, base_dir);

    // Determine active account from snapshot result (.current-account),
    // falling back to .csq-account marker.
    let account: AccountNum = match markers::read_current_account(&config_dir)
        .or_else(|| markers::read_csq_account(&config_dir))
    {
        Some(a) => a,
        None => {
            println!("csq: no active account");
            return Ok(());
        }
    };

    // ── Sync: backsync (live→canonical) + pullsync (canonical→live) ──
    // Best-effort, never blocks the statusline render.
    let _ = sync::backsync(&config_dir, base_dir);
    let _ = sync::pullsync(&config_dir, base_dir);

    // ── Render statusline (read-only from quota.json) ──
    // Quota data is written ONLY by the daemon's usage poller.
    let quota = state::load_state(base_dir).unwrap_or_else(|_| csq_core::quota::QuotaFile::empty());
    let account_quota = quota.get(account.get());

    let label = account_label(base_dir, account);
    let stuck = is_swap_stuck(&config_dir, base_dir);
    let broker_failed =
        should_report_broker_failed(base_dir, account) || is_broker_failed(base_dir, account);

    let line = statusline_str(account, &label, account_quota, stuck, broker_failed);
    println!("{line}");
    Ok(())
}
