use std::path::PathBuf;

/// launchd label for TelePi.
pub const LABEL: &str = "com.telepi";

/// Build a launchd plist for TelePi.
pub fn build_plist(
    telepi_bin: &str,
    config_path: &str,
    log_dir: &PathBuf,
) -> String {
    let out_log = log_dir.join("telepi.out.log").display().to_string();
    let err_log = log_dir.join("telepi.err.log").display().to_string();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>start</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>TELEPI_CONFIG</key>
        <string>{config}</string>
        <key>PATH</key>
        <string>{path}</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>{out_log}</string>
    <key>StandardErrorPath</key>
    <string>{err_log}</string>
</dict>
</plist>
"#,
        label = LABEL,
        bin = telepi_bin,
        config = config_path,
        path = std::env::var("PATH").unwrap_or_default(),
        out_log = out_log,
        err_log = err_log,
    )
}

/// Get the installed plist path.
pub fn installed_plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

/// Install the launchd agent: write plist and load it.
pub fn install(telepi_bin: &str, config_path: &str, log_dir: &PathBuf) -> Result<(), String> {
    let plist_content = build_plist(telepi_bin, config_path, log_dir);
    let plist_path = installed_plist_path();

    // Ensure parent directory exists
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create LaunchAgents dir: {e}"))?;
    }

    std::fs::write(&plist_path, plist_content)
        .map_err(|e| format!("failed to write plist: {e}"))?;

    // Load the agent
    let status = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .map_err(|e| format!("failed to run launchctl: {e}"))?;

    if !status.success() {
        return Err("launchctl load failed".into());
    }

    Ok(())
}

/// Uninstall the launchd agent: unload and remove plist.
pub fn uninstall() -> Result<(), String> {
    let plist_path = installed_plist_path();

    if plist_path.exists() {
        // Unload first (ignore errors if already unloaded)
        let _ = std::process::Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&plist_path)
            .status();

        std::fs::remove_file(&plist_path)
            .map_err(|e| format!("failed to remove plist: {e}"))?;
    }

    Ok(())
}

/// Check if the agent is currently loaded.
pub fn is_running() -> bool {
    let output = std::process::Command::new("launchctl")
        .args(["list"])
        .arg(LABEL)
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Stop the agent (unload without removing the plist).
pub fn stop() -> Result<(), String> {
    let plist_path = installed_plist_path();
    if !plist_path.exists() {
        return Err("service not installed".into());
    }

    let status = std::process::Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&plist_path)
        .status()
        .map_err(|e| format!("failed to run launchctl: {e}"))?;

    if !status.success() {
        return Err("launchctl unload failed".into());
    }

    Ok(())
}

/// Start the agent (load without reinstalling).
pub fn start() -> Result<(), String> {
    let plist_path = installed_plist_path();
    if !plist_path.exists() {
        return Err("service not installed — run `telepi gateway start` first".into());
    }

    let status = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .map_err(|e| format!("failed to run launchctl: {e}"))?;

    if !status.success() {
        return Err("launchctl load failed".into());
    }

    Ok(())
}
