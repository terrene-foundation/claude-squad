//! `csq swap N` — swap the active account in the current config directory.

use anyhow::Result;
use csq_core::rotation;
use csq_core::types::AccountNum;
use std::path::Path;

pub fn handle(base_dir: &Path, target: AccountNum) -> Result<()> {
    let config_dir = super::validated_config_dir(base_dir)?;

    let result = rotation::swap_to(base_dir, &config_dir, target)?;

    let expires_in_min = (result.expires_at_ms / 1000)
        .saturating_sub(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        )
        / 60;

    println!(
        "Swapped to account {} — token valid {}m",
        result.account, expires_in_min
    );
    Ok(())
}
