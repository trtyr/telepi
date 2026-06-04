use std::path::PathBuf;

use crate::error::{Result, TelePiError};
use crate::paths;

/// Tool output verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVerbosity {
    All,
    Summary,
    ErrorsOnly,
    None,
}

impl ToolVerbosity {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" => Self::All,
            "summary" => Self::Summary,
            "errors-only" | "errors_only" | "errorsonly" => Self::ErrorsOnly,
            "none" => Self::None,
            _ => Self::Summary,
        }
    }
}

impl std::fmt::Display for ToolVerbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Summary => write!(f, "summary"),
            Self::ErrorsOnly => write!(f, "errors-only"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Where the config file was found.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    Explicit(PathBuf),
    Cwd(PathBuf),
    Default(PathBuf),
    Missing,
}

/// Fully resolved TelePi configuration.
#[derive(Debug, Clone)]
pub struct TelePiConfig {
    pub telegram_bot_token: String,
    pub telegram_allowed_user_ids: Vec<u64>,
    pub workspace: PathBuf,
    pub tool_verbosity: ToolVerbosity,
    pub prompt_inbox_dir: Option<PathBuf>,
    pub prompt_inbox_interval_ms: u64,
    pub openai_api_key: Option<String>,
    pub sherpa_onnx_model_dir: Option<PathBuf>,
    pub sherpa_onnx_num_threads: u32,
    pub pi_session_path: Option<PathBuf>,
    pub pi_model: Option<String>,
    pub config_source: ConfigSource,
}

impl TelePiConfig {
    /// Whether a given Telegram user ID is allowed.
    pub fn is_allowed_user(&self, user_id: u64) -> bool {
        self.telegram_allowed_user_ids.contains(&user_id)
    }
}

/// Load configuration from environment variables and `.env` files.
///
/// Resolution order for config file:
///   1. `TELEPI_CONFIG` env var (explicit)
///   2. `.env` in current working directory
///   3. `~/.config/telepi/.env` (default)
pub fn load_config() -> Result<TelePiConfig> {
    // Try to load .env from the resolved config path
    let config_source = resolve_config_source();
    match &config_source {
        ConfigSource::Explicit(p) | ConfigSource::Cwd(p) | ConfigSource::Default(p) => {
            if p.exists() {
                dotenvy::from_path(p).ok();
            }
        }
        ConfigSource::Missing => {}
    }

    // Also try loading .env from cwd as fallback (dotenvy won't overwrite existing vars)
    dotenvy::dotenv().ok();

    let telegram_bot_token = require_env("TELEGRAM_BOT_TOKEN")?;
    let allowed_ids_raw = require_env("TELEGRAM_ALLOWED_USER_IDS")?;
    let telegram_allowed_user_ids = parse_allowed_user_ids(&allowed_ids_raw)?;

    let workspace = resolve_workspace();
    let tool_verbosity = optional_string("TOOL_VERBOSITY")
        .map(|s| ToolVerbosity::from_str_loose(&s))
        .unwrap_or(ToolVerbosity::Summary);

    let prompt_inbox_dir = optional_string("TELEPI_PROMPT_INBOX_DIR").map(PathBuf::from);
    let prompt_inbox_interval_ms = optional_string("TELEPI_PROMPT_INBOX_INTERVAL_MS")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60_000);

    let openai_api_key = optional_string("OPENAI_API_KEY");
    let sherpa_onnx_model_dir = optional_string("SHERPA_ONNX_MODEL_DIR").map(PathBuf::from);
    let sherpa_onnx_num_threads = optional_string("SHERPA_ONNX_NUM_THREADS")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(2);

    let pi_session_path = optional_string("PI_SESSION_PATH").map(PathBuf::from);
    let pi_model = optional_string("PI_MODEL");

    Ok(TelePiConfig {
        telegram_bot_token,
        telegram_allowed_user_ids,
        workspace,
        tool_verbosity,
        prompt_inbox_dir,
        prompt_inbox_interval_ms,
        openai_api_key,
        sherpa_onnx_model_dir,
        sherpa_onnx_num_threads,
        pi_session_path,
        pi_model,
        config_source,
    })
}

/// Parse comma-separated Telegram user IDs.
fn parse_allowed_user_ids(raw: &str) -> Result<Vec<u64>> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<u64>().map_err(|_| {
                TelePiError::InvalidConfig(format!(
                    "invalid Telegram user id in TELEGRAM_ALLOWED_USER_IDS: {s}"
                ))
            })
        })
        .collect()
}

/// Resolve workspace path.
fn resolve_workspace() -> PathBuf {
    // In Docker, use /workspace
    if PathBuf::from(paths::DOCKER_WORKSPACE_PATH).exists() {
        if let Ok(entries) = std::fs::read_dir(paths::DOCKER_WORKSPACE_PATH) {
            if entries.count() > 0 {
                return PathBuf::from(paths::DOCKER_WORKSPACE_PATH);
            }
        }
    }

    optional_string("TELEPI_WORKSPACE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Resolve config file source.
fn resolve_config_source() -> ConfigSource {
    // 1. Explicit env
    if let Some(explicit) = optional_string("TELEPI_CONFIG") {
        let p = paths::resolve_from_cwd(&explicit);
        return ConfigSource::Explicit(p);
    }

    // 2. .env in cwd
    let cwd_env = std::env::current_dir()
        .unwrap_or_default()
        .join(".env");
    if cwd_env.exists() {
        return ConfigSource::Cwd(cwd_env);
    }

    // 3. Default path
    let default = paths::default_config_path();
    if default.exists() {
        return ConfigSource::Default(default);
    }

    ConfigSource::Missing
}

/// Read a required environment variable.
fn require_env(name: &'static str) -> Result<String> {
    std::env::var(name).map_err(|_| TelePiError::MissingEnv(name))
}

/// Read an optional environment variable, returning None for empty/whitespace.
fn optional_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_allowed_user_ids() {
        let ids = parse_allowed_user_ids("123, 456,789").unwrap();
        assert_eq!(ids, vec![123, 456, 789]);
    }

    #[test]
    fn test_parse_allowed_user_ids_empty() {
        let ids = parse_allowed_user_ids("").unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_parse_allowed_user_ids_invalid() {
        assert!(parse_allowed_user_ids("abc").is_err());
    }

    #[test]
    fn test_tool_verbosity_from_str() {
        assert_eq!(ToolVerbosity::from_str_loose("all"), ToolVerbosity::All);
        assert_eq!(ToolVerbosity::from_str_loose("SUMMARY"), ToolVerbosity::Summary);
        assert_eq!(ToolVerbosity::from_str_loose("errors-only"), ToolVerbosity::ErrorsOnly);
        assert_eq!(ToolVerbosity::from_str_loose("none"), ToolVerbosity::None);
        assert_eq!(ToolVerbosity::from_str_loose("garbage"), ToolVerbosity::Summary);
    }
}
