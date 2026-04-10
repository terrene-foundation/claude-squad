//! `csq models [provider]` — list models grouped by provider.

use anyhow::Result;
use csq_core::providers::{self, ModelCatalog};
use std::path::Path;

pub fn handle(_base_dir: &Path, provider_filter: &str) -> Result<()> {
    let catalog = ModelCatalog::default_catalog();

    println!();

    if provider_filter == "all" {
        for provider in providers::PROVIDERS {
            let models: Vec<_> = catalog.by_provider(provider.id).into_iter().collect();
            if models.is_empty() && provider.id != "ollama" {
                continue;
            }
            println!("{} ({})", provider.name, provider.id);
            for m in &models {
                println!("  {} — {}", m.id, m.name);
            }
            if provider.id == "ollama" {
                // Live query ollama
                let live = providers::ollama::get_ollama_models();
                if live.is_empty() && models.is_empty() {
                    println!("  (ollama not installed or no models)");
                } else {
                    for name in &live {
                        println!("  {name}");
                    }
                }
            }
            println!();
        }
    } else {
        let provider = providers::get_provider(provider_filter);
        if provider.is_none() {
            return Err(anyhow::anyhow!("unknown provider: {provider_filter}"));
        }

        let p = provider.unwrap();
        println!("{} ({})", p.name, p.id);
        let models = catalog.by_provider(p.id);
        for m in &models {
            println!("  {} — {}", m.id, m.name);
        }
        if p.id == "ollama" {
            for name in providers::ollama::get_ollama_models() {
                println!("  {name}");
            }
        }
        println!();
    }

    Ok(())
}
