use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use telepi::cli::{Cli, Commands, GatewayCommand};
use telepi::config;

/// Kill any existing telepi processes to ensure a clean start.
fn kill_existing_processes() {
    use std::process::Command;

    let self_pid = std::process::id();
    let Ok(output) = Command::new("pgrep").arg("-f").arg("telepi").output() else {
        return;
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(pid) = line.trim().parse::<u32>() {
            if pid != self_pid {
                eprintln!("killing previous telepi process (pid {pid})...");
                let _ = Command::new("kill").arg(pid.to_string()).status();
            }
        }
    }

    // Also kill any orphan pi CLI children
    let _ = Command::new("pkill").arg("-f").arg("pi --mode json").status();

    // Wait for processes to die
    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start) | None => {
            let cfg = config::load_config()?;

            // Clean up any previous instances
            kill_existing_processes();

            // Apply proxy from config — must happen before reqwest reads env
            if let Some(proxy) = &cfg.proxy {
                // SAFETY: single-threaded before tokio runtime starts
                unsafe {
                    std::env::set_var("HTTP_PROXY", proxy);
                    std::env::set_var("HTTPS_PROXY", proxy);
                }
            }

            // Build tokio runtime
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;

            rt.block_on(async {
                // Initialize logging with config log level
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::new(&cfg.log_level))
                    .init();

                info!(proxy = ?cfg.proxy, log_level = %cfg.log_level, "loading configuration...");
                info!(
                    workspace = %cfg.workspace.display(),
                    allowed_users = cfg.telegram_allowed_user_ids.len(),
                    "configuration loaded"
                );
                telepi::bot::run(cfg).await
            })?;
        }
        Some(Commands::Setup {
            bot_token: _,
            user_ids: _,
            workspace: _,
        }) => {
            println!("⚠️  Interactive setup not yet implemented.");
            println!("Create a telepi.toml file manually:");
            println!(r#"
proxy = "http://127.0.0.1:7890"
log_level = "info"

[telegram]
bot_token = "your-bot-token"
allowed_user_ids = [your-user-id]

[pi]
tool_verbosity = "summary"
"#);
        }
        Some(Commands::Gateway { command }) => {
            match command {
                GatewayCommand::Start => {
                    let telepi_bin = std::env::current_exe()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "telepi".into());
                    let config_path = telepi::paths::default_config_path()
                        .display().to_string();
                    let log_dir = telepi::paths::default_log_dir();

                    if telepi::install::gateway_is_running() {
                        println!("gateway is already running.");
                        return Ok(());
                    }

                    match telepi::install::gateway_install(&telepi_bin, &config_path, &log_dir) {
                        Ok(()) => println!("✅ gateway started."),
                        Err(e) => eprintln!("❌ failed to start gateway: {e}"),
                    }
                }
                GatewayCommand::Stop => {
                    if !telepi::install::gateway_is_running() {
                        println!("gateway is not running.");
                        return Ok(());
                    }

                    match telepi::install::gateway_stop() {
                        Ok(()) => println!("✅ gateway stopped."),
                        Err(e) => eprintln!("❌ failed to stop gateway: {e}"),
                    }
                }
                GatewayCommand::Restart => {
                    let _ = telepi::install::gateway_stop();
                    std::thread::sleep(std::time::Duration::from_secs(1));

                    let telepi_bin = std::env::current_exe()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "telepi".into());
                    let config_path = telepi::paths::default_config_path()
                        .display().to_string();
                    let log_dir = telepi::paths::default_log_dir();

                    match telepi::install::gateway_install(&telepi_bin, &config_path, &log_dir) {
                        Ok(()) => println!("✅ gateway restarted."),
                        Err(e) => eprintln!("❌ failed to restart gateway: {e}"),
                    }
                }
            }
        }
        Some(Commands::Status) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            rt.block_on(async {
                let status = telepi::install::get_status().await;
                println!("TelePi v{}", status.version);
                println!();
                match &status.config_path {
                    Some(p) => println!("Config: {}", p.display()),
                    None => println!("Config: not found"),
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
            });
        }
    }

    Ok(())
}
