//! `csq install` — set up the `~/.claude/accounts` directory and patch settings.

use anyhow::{Context, Result};
use std::path::Path;

pub fn handle() -> Result<()> {
    let base_dir = super::base_dir()?;
    let claude_home = super::claude_home()?;

    println!("Installing csq...");
    println!();

    // Create directories
    let credentials_dir = base_dir.join("credentials");
    std::fs::create_dir_all(&credentials_dir)
        .context("creating credentials directory")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&base_dir, std::fs::Permissions::from_mode(0o700))?;
        std::fs::set_permissions(&credentials_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    println!("  ✓ Created {}", base_dir.display());
    println!("  ✓ Created {}", credentials_dir.display());

    // Patch settings.json
    patch_settings_json(&claude_home)?;
    println!("  ✓ Patched {}/settings.json", claude_home.display());

    // Clean up v1.x artifacts
    let cleaned = cleanup_v1_artifacts(&claude_home);
    if !cleaned.is_empty() {
        println!();
        println!("  Cleaned v1.x artifacts:");
        for item in &cleaned {
            println!("    - {item}");
        }
    }

    println!();
    println!("csq installed successfully.");
    println!();
    println!("Next steps:");
    println!("  1. Run `csq login 1` to authenticate your first account");
    println!("  2. Run `csq status` to verify");
    println!("  3. Run `csq run 1` to start a Claude Code session");

    Ok(())
}

fn patch_settings_json(claude_home: &Path) -> Result<()> {
    let path = claude_home.join("settings.json");

    std::fs::create_dir_all(claude_home)?;

    let mut value: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(content) if !content.trim().is_empty() => serde_json::from_str(&content)
            .unwrap_or_else(|_| serde_json::json!({})),
        _ => serde_json::json!({}),
    };

    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "statusLineCommand".to_string(),
            serde_json::json!("csq statusline"),
        );
    }

    let json = serde_json::to_string_pretty(&value)?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn cleanup_v1_artifacts(claude_home: &Path) -> Vec<String> {
    let mut cleaned = Vec::new();
    let v1_files = [
        "statusline-command.sh",
        "rotate.md",
        "auto-rotate-hook.sh",
    ];

    for name in &v1_files {
        let path = claude_home.join(name);
        if path.exists() {
            // Rename to .bak instead of deleting, per safety policy
            let bak = path.with_extension(
                format!("{}.bak", path.extension().and_then(|e| e.to_str()).unwrap_or(""))
                    .trim_start_matches('.'),
            );
            if std::fs::rename(&path, &bak).is_ok() {
                cleaned.push(name.to_string());
            }
        }
    }

    cleaned
}
