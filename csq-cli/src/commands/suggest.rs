//! `csq suggest` — JSON output of the best account to switch to.

use anyhow::Result;
use csq_core::accounts::markers;
use csq_core::rotation;
use std::path::Path;

pub fn handle(base_dir: &Path) -> Result<()> {
    let current = super::current_config_dir()
        .as_deref()
        .and_then(markers::read_current_account);

    let suggestion = rotation::suggest(base_dir, current);
    println!("{}", serde_json::to_string(&suggestion)?);
    Ok(())
}
