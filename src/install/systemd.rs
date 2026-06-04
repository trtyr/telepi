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
