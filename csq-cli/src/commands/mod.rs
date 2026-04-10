//! Subcommand handlers for the csq CLI.

pub mod install;
pub mod listkeys;
pub mod login;
pub mod models;
pub mod rmkey;
pub mod run;
pub mod setkey;
pub mod status;
pub mod statusline;
pub mod suggest;
pub mod swap;

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Returns the base directory for csq state: `~/.claude/accounts`.
///
/// Honors `CSQ_BASE_DIR` environment variable for testing.
pub fn base_dir() -> Result<PathBuf> {
    if let Ok(override_path) = std::env::var("CSQ_BASE_DIR") {
        return Ok(PathBuf::from(override_path));
    }

    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".claude").join("accounts"))
}

/// Returns the user's `~/.claude` directory (CC's config home).
pub fn claude_home() -> Result<PathBuf> {
    if let Ok(override_path) = std::env::var("CLAUDE_HOME") {
        return Ok(PathBuf::from(override_path));
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".claude"))
}

/// Returns the current working directory as a config dir if it matches `config-*`,
/// or the `CLAUDE_CONFIG_DIR` env var if set.
pub fn current_config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    None
}
