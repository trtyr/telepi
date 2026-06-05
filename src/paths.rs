use std::path::PathBuf;

/// Docker workspace mount path.
pub const DOCKER_WORKSPACE_PATH: &str = "/workspace";

/// Get the user's home directory.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("could not determine home directory")
}

/// Expand `~` at the start of a path.
pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home_dir().join(rest)
    } else if path == "~" {
        home_dir()
    } else {
        PathBuf::from(path)
    }
}

/// Resolve a path relative to cwd.
pub fn resolve_from_cwd(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p)
    }
}

/// Default TelePi config directory: `~/.config/telepi/`
pub fn default_config_dir() -> PathBuf {
    home_dir().join(".config").join("telepi")
}

/// Default config file path: `~/.config/telepi/config.toml`
pub fn default_config_path() -> PathBuf {
    default_config_dir().join("config.toml")
}

/// Default systemd user unit directory: `~/.config/systemd/user/`
pub fn default_systemd_user_dir() -> PathBuf {
    home_dir().join(".config").join("systemd").join("user")
}

/// Default log directory: `~/Library/Logs/TelePi/` (macOS) or `~/.local/state/telepi/logs/` (Linux).
pub fn default_log_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home_dir().join("Library").join("Logs").join("TelePi")
    } else {
        home_dir().join(".local").join("state").join("telepi").join("logs")
    }
}
