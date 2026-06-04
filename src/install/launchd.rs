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
