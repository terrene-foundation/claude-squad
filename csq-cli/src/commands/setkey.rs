//! `csq setkey <provider> --key <KEY>` — set a provider's API key.
//!
//! If `--key` is not provided, reads from stdin (keeps the key out of
//! shell history).

use anyhow::{anyhow, Result};
use csq_core::providers;
use std::io::Read;
use std::path::Path;

pub fn handle(base_dir: &Path, provider_id: &str, key_arg: Option<&str>) -> Result<()> {
    if providers::get_provider(provider_id).is_none() {
        return Err(anyhow!("unknown provider: {provider_id}"));
    }

    let key = match key_arg {
        Some(k) => k.trim().to_string(),
        None => read_key_from_stdin()?,
    };

    if key.is_empty() {
        return Err(anyhow!("key is empty"));
    }

    // Strip \r for Windows clipboard paste
    let key = key.trim_end_matches('\r').to_string();

    let mut settings = providers::settings::load_settings(base_dir, provider_id)?;
    settings.set_api_key(&key)?;
    providers::settings::save_settings(base_dir, &settings)?;

    println!(
        "Set {} key: {}",
        provider_id,
        settings.key_fingerprint()
    );
    Ok(())
}

fn read_key_from_stdin() -> Result<String> {
    println!("Enter API key (paste, then Enter):");
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf.trim().to_string())
}
