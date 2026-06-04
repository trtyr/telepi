/// Supported platforms for service installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MacOs => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
        }
    }
}

/// Detect the current platform.
pub fn detect_platform() -> Option<Platform> {
    if cfg!(target_os = "macos") {
        Some(Platform::MacOs)
    } else if cfg!(target_os = "linux") {
        Some(Platform::Linux)
    } else {
        None
    }
}

/// Status of the installed service.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub installed: bool,
    pub running: bool,
    pub platform: Platform,
    pub unit_path: Option<std::path::PathBuf>,
}

/// Status of the Pi handoff extension.
#[derive(Debug, Clone)]
pub struct ExtensionStatus {
    pub installed: bool,
    pub path: Option<std::path::PathBuf>,
    pub method: Option<&'static str>,
}

/// Overall TelePi installation status.
#[derive(Debug, Clone)]
pub struct TelePiStatus {
    pub version: String,
    pub config_path: Option<std::path::PathBuf>,
    pub service: Option<ServiceStatus>,
    pub extension: Option<ExtensionStatus>,
}
