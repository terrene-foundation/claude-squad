//! csq v2.0 CLI entry point.
//!
//! Routes subcommands to handlers in `commands/`. Single binary replaces
//! the v1.x bash + Python toolchain.

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use csq_core::types::AccountNum;
use tracing_subscriber::EnvFilter;

/// csq — Claude Code multi-account rotation and session management
#[derive(Parser, Debug)]
#[command(name = "csq", version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Positional account number — shorthand for `csq run <N>`
    #[arg(value_name = "ACCOUNT")]
    account: Option<u16>,

    /// Remaining args passed through to `claude`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    rest: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run claude with an isolated config directory for the given account
    Run {
        /// Account number (1-999)
        account: Option<u16>,
        /// Optional profile (overrides credentials with a provider settings file)
        #[arg(short, long)]
        profile: Option<String>,
        /// Arguments passed through to `claude`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },

    /// Swap the active account in the current config dir
    Swap {
        /// Account number to swap to
        account: u16,
    },

    /// Show status of all accounts
    Status,

    /// Suggest the best account to switch to (JSON output)
    Suggest,

    /// Show the statusline string (reads CC JSON from stdin)
    Statusline,

    /// OAuth login flow for a new account
    Login {
        /// Account number to login as
        account: u16,
    },

    /// Provider key management
    #[command(subcommand)]
    Setkey(SetkeyCmd),

    /// List configured provider keys
    Listkeys,

    /// Remove a provider key
    Rmkey {
        /// Provider ID (mm, zai, etc.)
        provider: String,
    },

    /// Model catalog operations
    Models {
        #[command(subcommand)]
        action: Option<ModelsCmd>,
    },

    /// Install csq into ~/.claude (creates dirs, patches settings.json)
    Install,
}

#[derive(Subcommand, Debug)]
enum SetkeyCmd {
    /// MiniMax API key
    Mm {
        #[arg(long)]
        key: Option<String>,
    },
    /// Z.AI API key
    Zai {
        #[arg(long)]
        key: Option<String>,
    },
    /// Claude API key (for non-OAuth flows)
    Claude {
        #[arg(long)]
        key: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ModelsCmd {
    /// List all models, or filter by provider
    List {
        /// Provider ID or "all"
        #[arg(default_value = "all")]
        provider: String,
    },
    /// Switch the active model for a provider
    Switch {
        /// Provider ID (claude, mm, zai, ollama)
        provider: String,
        /// Model ID or alias
        model: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("CSQ_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // No subcommand: default to `run` (optionally with positional account)
    let command = cli.command.unwrap_or(Command::Run {
        account: cli.account,
        profile: None,
        rest: cli.rest,
    });

    let base_dir = commands::base_dir()?;

    match command {
        Command::Run { account, profile, rest } => {
            let account_num = match account {
                Some(n) => Some(
                    AccountNum::try_from(n)
                        .map_err(|e| anyhow::anyhow!("invalid account: {e}"))?,
                ),
                None => None,
            };
            commands::run::handle(&base_dir, account_num, profile.as_deref(), &rest)
        }
        Command::Swap { account } => {
            let account_num = AccountNum::try_from(account)
                .map_err(|e| anyhow::anyhow!("invalid account: {e}"))?;
            commands::swap::handle(&base_dir, account_num)
        }
        Command::Status => commands::status::handle(&base_dir),
        Command::Suggest => commands::suggest::handle(&base_dir),
        Command::Statusline => commands::statusline::handle(&base_dir),
        Command::Login { account } => {
            let account_num = AccountNum::try_from(account)
                .map_err(|e| anyhow::anyhow!("invalid account: {e}"))?;
            commands::login::handle(&base_dir, account_num)
        }
        Command::Setkey(sk) => {
            let (provider, key) = match sk {
                SetkeyCmd::Mm { key } => ("mm", key),
                SetkeyCmd::Zai { key } => ("zai", key),
                SetkeyCmd::Claude { key } => ("claude", key),
            };
            commands::setkey::handle(&base_dir, provider, key.as_deref())
        }
        Command::Listkeys => commands::listkeys::handle(&base_dir),
        Command::Rmkey { provider } => commands::rmkey::handle(&base_dir, &provider),
        Command::Models { action } => {
            let action = action.unwrap_or(ModelsCmd::List {
                provider: "all".to_string(),
            });
            match action {
                ModelsCmd::List { provider } => commands::models::handle_list(&base_dir, &provider),
                ModelsCmd::Switch { provider, model } => {
                    commands::models::handle_switch(&base_dir, &provider, &model)
                }
            }
        }
        Command::Install => commands::install::handle(),
    }
}
