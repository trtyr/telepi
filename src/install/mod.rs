pub mod launchd;
pub mod platform;
pub mod systemd;

use crate::install::platform::{ExtensionStatus, Platform, ServiceStatus, TelePiStatus};

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
