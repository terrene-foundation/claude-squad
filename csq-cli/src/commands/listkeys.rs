//! `csq listkeys` — list configured provider keys.

use anyhow::Result;
use csq_core::providers;
use std::path::Path;

pub fn handle(base_dir: &Path) -> Result<()> {
    let configured = providers::settings::list_configured(base_dir);

    if configured.is_empty() {
        println!("No provider keys configured.");
        println!();
        println!("Run `csq setkey mm --key <KEY>` to add a MiniMax key, for example.");
        return Ok(());
    }

    println!();
    println!("Configured provider keys:");
    println!();

    for s in configured {
        let provider = providers::get_provider(&s.provider_id);
        let name = provider.map(|p| p.name).unwrap_or(&s.provider_id);
        let fp = s.key_fingerprint();
        let model = s.get_model().unwrap_or("(default)");

        let path = providers::settings::settings_path(base_dir, &s.provider_id)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(unknown)".into());

        println!("  {} ({})", name, s.provider_id);
        println!("    Key:    {fp}");
        println!("    Model:  {model}");
        println!("    File:   {path}");
        println!();
    }

    Ok(())
}
