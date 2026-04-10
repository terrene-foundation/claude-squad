//! `csq install` — set up the `~/.claude/accounts` directory and patch settings.

use anyhow::{anyhow, Context, Result};
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

    // Read existing settings.
    // On parse failure, DO NOT silently replace — refuse to run and
    // ask the user to repair the file manually. This prevents data loss
    // of user's MCP servers, hooks, and custom permissions.
    let mut value: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(content) if !content.trim().is_empty() => {
            serde_json::from_str(&content).map_err(|e| {
                anyhow!(
                    "failed to parse existing {} ({e}).\n\
                     Refusing to overwrite to prevent data loss.\n\
                     Fix the JSON manually and re-run `csq install`.",
                    path.display()
                )
            })?
        }
        _ => serde_json::json!({}),
    };

    // Ensure top-level is an object.
    let obj = value.as_object_mut().ok_or_else(|| {
        anyhow!("{} is not a JSON object", path.display())
    })?;

    // Insert the statusLine using CC's expected NESTED schema:
    //   { "statusLine": { "type": "command", "command": "csq statusline" } }
    // The flat `statusLineCommand` key would never be read by CC.
    obj.insert(
        "statusLine".to_string(),
        serde_json::json!({
            "type": "command",
            "command": "csq statusline"
        }),
    );

    // Atomic write via temp file + rename
    let json = serde_json::to_string_pretty(&value)?;
    let tmp = csq_core::platform::fs::unique_tmp_path(&path);
    std::fs::write(&tmp, json.as_bytes())
        .with_context(|| format!("writing temp file {}", tmp.display()))?;
    csq_core::platform::fs::atomic_replace(&tmp, &path)
        .map_err(|e| anyhow!("atomic replace: {e}"))?;
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
