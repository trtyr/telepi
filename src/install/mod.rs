pub mod launchd;
pub mod platform;
pub mod systemd;

use crate::install::platform::{ExtensionStatus, Platform, ServiceStatus, TelePiStatus};

/// Install and start the gateway service for the current platform.
pub fn gateway_install(telepi_bin: &str, config_path: &str, log_dir: &std::path::PathBuf) -> Result<(), String> {
    let platform = platform::detect_platform()
        .ok_or("unsupported platform — only macOS and Linux are supported")?;

    match platform {
        Platform::MacOs => launchd::install(telepi_bin, config_path, log_dir),
        Platform::Linux => systemd::install(telepi_bin, config_path, log_dir),
    }
}

/// Stop the gateway service.
pub fn gateway_stop() -> Result<(), String> {
    let platform = platform::detect_platform()
        .ok_or("unsupported platform")?;

    match platform {
        Platform::MacOs => launchd::stop(),
        Platform::Linux => systemd::stop(),
    }
}

/// Start the gateway service (without reinstalling).
pub fn gateway_start() -> Result<(), String> {
    let platform = platform::detect_platform()
        .ok_or("unsupported platform")?;

    match platform {
        Platform::MacOs => launchd::start(),
        Platform::Linux => systemd::start(),
    }
}

/// Uninstall the gateway service.
pub fn gateway_uninstall() -> Result<(), String> {
    let platform = platform::detect_platform()
        .ok_or("unsupported platform")?;

    match platform {
        Platform::MacOs => launchd::uninstall(),
        Platform::Linux => systemd::uninstall(),
    }
}

/// Check if the gateway service is currently running.
pub fn gateway_is_running() -> bool {
    let Some(platform) = platform::detect_platform() else {
        return false;
    };

    match platform {
        Platform::MacOs => launchd::is_running(),
        Platform::Linux => systemd::is_running(),
    }
}

/// Get the current TelePi installation status.
pub async fn get_status() -> TelePiStatus {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let config_path = crate::paths::default_config_path();
    let config_path = if config_path.exists() {
        Some(config_path)
    } else {
        None
    };

    let platform = platform::detect_platform();
    let service = platform.map(|p| {
        let (installed, unit_path) = match p {
            Platform::MacOs => {
                let path = launchd::installed_plist_path();
                (path.exists(), Some(path))
            }
            Platform::Linux => {
                let path = systemd::installed_unit_path();
                (path.exists(), Some(path))
            }
        };

        // TODO: Check if service is actually running
        ServiceStatus {
            installed,
            running: false,
            platform: p,
            unit_path,
        }
    });

    // Check extension status
    let extension = check_extension_status();

    TelePiStatus {
        version,
        config_path,
        service,
        extension,
    }
}

/// Check if the Pi handoff extension is installed.
fn check_extension_status() -> Option<ExtensionStatus> {
    // Look for the extension in common locations
    let pi_ext_dir = crate::paths::home_dir()
        .join(".pi")
        .join("agent")
        .join("extensions");

    let ext_file = pi_ext_dir.join("telepi-handoff.ts");

    if ext_file.exists() {
        Some(ExtensionStatus {
            installed: true,
            path: Some(ext_file),
            method: Some("file"),
        })
    } else {
        Some(ExtensionStatus {
            installed: false,
            path: None,
            method: None,
        })
    }
}
