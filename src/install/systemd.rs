use std::path::PathBuf;

/// systemd service name for TelePi.
pub const SERVICE_NAME: &str = "telepi";

/// Build a systemd user unit file for TelePi.
pub fn build_unit(
    telepi_bin: &str,
    config_path: &str,
    log_dir: &PathBuf,
) -> String {
    let out_log = log_dir.join("telepi.out.log").display().to_string();
    let err_log = log_dir.join("telepi.err.log").display().to_string();

    format!(
        r#"[Unit]
Description=TelePi — Telegram bridge for the Pi coding agent
After=network.target

[Service]
Type=simple
ExecStart={bin} start
Environment=TELEPI_CONFIG={config}
Restart=on-failure
RestartSec=5
StandardOutput=append:{out_log}
StandardError=append:{err_log}

[Install]
WantedBy=default.target
"#,
        bin = telepi_bin,
        config = config_path,
        out_log = out_log,
        err_log = err_log,
    )
}

/// Get the installed unit file path.
pub fn installed_unit_path() -> PathBuf {
    crate::paths::default_systemd_user_dir().join(format!("{SERVICE_NAME}.service"))
}

/// Install the systemd user service: write unit file, enable and start it.
pub fn install(telepi_bin: &str, config_path: &str, log_dir: &PathBuf) -> Result<(), String> {
    let unit_content = build_unit(telepi_bin, config_path, log_dir);
    let unit_path = installed_unit_path();

    if let Some(parent) = unit_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create systemd user dir: {e}"))?;
    }

    std::fs::write(&unit_path, unit_content)
        .map_err(|e| format!("failed to write unit file: {e}"))?;

    // Reload, enable, and start
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    let status = std::process::Command::new("systemctl")
        .args(["--user", "enable", "--now", &format!("{SERVICE_NAME}.service")])
        .status()
        .map_err(|e| format!("failed to run systemctl: {e}"))?;

    if !status.success() {
        return Err("systemctl enable --now failed".into());
    }

    Ok(())
}

/// Uninstall the systemd user service: stop, disable, and remove unit file.
pub fn uninstall() -> Result<(), String> {
    let unit_path = installed_unit_path();

    if unit_path.exists() {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "--now", &format!("{SERVICE_NAME}.service")])
            .status();

        std::fs::remove_file(&unit_path)
            .map_err(|e| format!("failed to remove unit file: {e}"))?;

        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
    }

    Ok(())
}

/// Check if the service is currently active.
pub fn is_running() -> bool {
    let output = std::process::Command::new("systemctl")
        .args(["--user", "is-active", &format!("{SERVICE_NAME}.service")])
        .output();

    match output {
        Ok(out) => {
            let status = String::from_utf8_lossy(&out.stdout);
            status.trim() == "active"
        }
        Err(_) => false,
    }
}

/// Stop the service without removing the unit file.
pub fn stop() -> Result<(), String> {
    let unit_path = installed_unit_path();
    if !unit_path.exists() {
        return Err("service not installed".into());
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "stop", &format!("{SERVICE_NAME}.service")])
        .status()
        .map_err(|e| format!("failed to run systemctl: {e}"))?;

    if !status.success() {
        return Err("systemctl stop failed".into());
    }

    Ok(())
}

/// Start the service without reinstalling.
pub fn start() -> Result<(), String> {
    let unit_path = installed_unit_path();
    if !unit_path.exists() {
        return Err("service not installed — run `telepi gateway start` first".into());
    }

    let status = std::process::Command::new("systemctl")
        .args(["--user", "start", &format!("{SERVICE_NAME}.service")])
        .status()
        .map_err(|e| format!("failed to run systemctl: {e}"))?;

    if !status.success() {
        return Err("systemctl start failed".into());
    }

    Ok(())
}
