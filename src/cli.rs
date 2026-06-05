use clap::{Parser, Subcommand};

/// TelePi — Telegram bridge for the Pi coding agent.
#[derive(Parser, Debug)]
#[command(name = "telepi", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Telegram bot
    Start,
    /// Interactive setup (bot token, user IDs, workspace)
    Setup {
        /// Pre-filled bot token (non-interactive)
        bot_token: Option<String>,
        /// Pre-filled allowed user IDs (non-interactive)
        user_ids: Option<String>,
        /// Pre-filled workspace path (non-interactive)
        workspace: Option<String>,
    },
    /// Show installed-mode status
    Status,
    /// Manage the background gateway service
    Gateway {
        #[command(subcommand)]
        command: GatewayCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum GatewayCommand {
    /// Install and start the gateway service
    Start,
    /// Stop the gateway service
    Stop,
    /// Restart the gateway service
    Restart,
}
