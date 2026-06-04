use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use telepi::cli::{Cli, Commands};
use telepi::config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start) | None => {
            info!("loading configuration...");
            let cfg = config::load_config()?;
            info!(
                workspace = %cfg.workspace.display(),
                allowed_users = cfg.telegram_allowed_user_ids.len(),
                "configuration loaded"
            );
            telepi::bot::run(cfg).await?;
        }
        Some(Commands::Setup {
            bot_token: _,
            user_ids: _,
            workspace: _,
        }) => {
            // TODO: Interactive setup
            println!("⚠️  Interactive setup not yet implemented.");
            println!("Create a .env file manually with:");
            println!("  TELEGRAM_BOT_TOKEN=<your-bot-token>");
            println!("  TELEGRAM_ALLOWED_USER_IDS=<your-user-id>");
            println!("  TELEPI_WORKSPACE=<your-workspace-path>");
        }
        Some(Commands::Status) => {
            let status = telepi::install::get_status().await;
            println!("TelePi v{}", status.version);
            println!();
            match &status.config_path {
                Some(p) => println!("Config: {}", p.display()),
                None => println!("Config: not found (run `telepi setup`)"),
            }
            if let Some(svc) = &status.service {
                println!("Platform: {}", svc.platform);
                println!(
                    "Service: {}",
                    if svc.running {
                        "running"
                    } else if svc.installed {
                        "installed (stopped)"
                    } else {
                        "not installed"
                    }
                );
                if let Some(path) = &svc.unit_path {
                    println!("Unit: {}", path.display());
                }
            }
            if let Some(ext) = &status.extension {
                println!(
                    "Extension: {}",
                    if ext.installed { "installed" } else { "not installed" }
                );
                if let Some(path) = &ext.path {
                    println!("Extension path: {}", path.display());
                }
            }
        }
    }

    Ok(())
}
