use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Result, TelePiError};
use crate::paths;

// ─── Tool Verbosity ──────────────────────────────────────────────────────────

/// Tool output verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolVerbosity {
    All,
    Summary,
    ErrorsOnly,
    None,
}

impl Default for ToolVerbosity {
    fn default() -> Self {
        Self::Summary
    }
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

// ─── TOML Config Structures ──────────────────────────────────────────────────

/// Top-level TOML config file structure.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TomlConfig {
    pub telegram: TelegramSection,
    pub pi: PiSection,
    pub prompt_inbox: PromptInboxSection,
    pub voice: VoiceSection,
    pub proxy: Option<String>,
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TelegramSection {
    pub bot_token: Option<String>,
    pub allowed_user_ids: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PiSection {
    pub workspace: Option<PathBuf>,
    pub model: Option<String>,
    pub session_path: Option<PathBuf>,
    pub tool_verbosity: Option<ToolVerbosity>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PromptInboxSection {
    pub dir: Option<PathBuf>,
    pub interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct VoiceSection {
    pub openai_api_key: Option<String>,
    pub sherpa_onnx_model_dir: Option<PathBuf>,
    pub sherpa_onnx_num_threads: Option<u32>,
}

// ─── Resolved Config ─────────────────────────────────────────────────────────

/// Where the config file was found.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    Toml(PathBuf),
    EnvOnly,
    Missing,
}

/// Fully resolved TelePi configuration.
#[derive(Debug, Clone)]
pub struct TelePiConfig {
    pub telegram_bot_token: String,
    pub telegram_allowed_user_ids: Vec<u64>,
    pub workspace: PathBuf,
    pub proxy: Option<String>,
    pub log_level: String,
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

// ─── Config Loading ──────────────────────────────────────────────────────────

/// Load configuration from a TOML config file with optional env var overrides.
///
/// Resolution order for config file:
///   1. `TELEPI_CONFIG` env var (explicit path to `.toml` file)
///   2. `./telepi.toml` in current working directory
///   3. `~/.pi/telepi/config.toml` (default)
///
/// After loading the TOML file, specific fields can be overridden by env vars.
pub fn load_config() -> Result<TelePiConfig> {
    let (toml_config, config_source) = load_toml_config()?;

    // Build resolved config, with env vars overriding TOML values
    let telegram_bot_token = env_override("TELEGRAM_BOT_TOKEN")
        .or(toml_config.telegram.bot_token)
        .ok_or(TelePiError::InvalidConfig("missing required field: telegram.bot_token".into()))?;

    let telegram_allowed_user_ids = {
        let raw = env_override("TELEGRAM_ALLOWED_USER_IDS");
        if let Some(raw) = raw {
            parse_allowed_user_ids(&raw)?
        } else if !toml_config.telegram.allowed_user_ids.is_empty() {
            toml_config.telegram.allowed_user_ids
        } else {
            return Err(TelePiError::InvalidConfig(
                "missing required field: telegram.allowed_user_ids".into(),
            ));
        }
    };

    let workspace = env_override("TELEPI_WORKSPACE")
        .map(PathBuf::from)
        .or(toml_config.pi.workspace)
        .or_else(|| {
            let docker_ws = PathBuf::from(paths::DOCKER_WORKSPACE_PATH);
            if docker_ws.exists() {
                Some(docker_ws)
            } else {
                None
            }
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let tool_verbosity = env_override("TOOL_VERBOSITY")
        .map(|s| ToolVerbosity::from_str_loose(&s))
        .or(toml_config.pi.tool_verbosity)
        .unwrap_or_default();

    let prompt_inbox_dir = env_override("TELEPI_PROMPT_INBOX_DIR")
        .map(PathBuf::from)
        .or(toml_config.prompt_inbox.dir);

    let prompt_inbox_interval_ms = env_override("TELEPI_PROMPT_INBOX_INTERVAL_MS")
        .and_then(|s| s.parse::<u64>().ok())
        .or(toml_config.prompt_inbox.interval_ms)
        .unwrap_or(60_000);

    let openai_api_key = env_override("OPENAI_API_KEY")
        .or(toml_config.voice.openai_api_key);

    let sherpa_onnx_model_dir = env_override("SHERPA_ONNX_MODEL_DIR")
        .map(PathBuf::from)
        .or(toml_config.voice.sherpa_onnx_model_dir);

    let sherpa_onnx_num_threads = env_override("SHERPA_ONNX_NUM_THREADS")
        .and_then(|s| s.parse::<u32>().ok())
        .or(toml_config.voice.sherpa_onnx_num_threads)
        .unwrap_or(2);

    let pi_session_path = env_override("PI_SESSION_PATH")
        .map(PathBuf::from)
        .or(toml_config.pi.session_path);

    let pi_model = env_override("PI_MODEL")
        .or(toml_config.pi.model);

    let proxy = env_override("HTTP_PROXY")
        .or(env_override("HTTPS_PROXY"))
        .or(env_override("ALL_PROXY"))
        .or(toml_config.proxy);

    let log_level = env_override("RUST_LOG")
        .or(toml_config.log_level)
        .unwrap_or_else(|| "info".to_string());

    Ok(TelePiConfig {
        telegram_bot_token,
        telegram_allowed_user_ids,
        workspace,
        proxy,
        log_level,
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

// ─── TOML File Resolution ────────────────────────────────────────────────────

/// Find and parse the TOML config file.
fn load_toml_config() -> Result<(TomlConfig, ConfigSource)> {
    let config_path = resolve_toml_path();

    match config_path {
        Some(path) => {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                TelePiError::InvalidConfig(format!(
                    "failed to read config file {}: {e}",
                    path.display()
                ))
            })?;
            let config: TomlConfig = toml::from_str(&content).map_err(|e| {
                TelePiError::InvalidConfig(format!(
                    "failed to parse config file {}: {e}",
                    path.display()
                ))
            })?;
            Ok((config, ConfigSource::Toml(path)))
        }
        None => {
            // No TOML file found — try .env as legacy fallback
            dotenvy::dotenv().ok();
            Ok((TomlConfig::default(), ConfigSource::Missing))
        }
    }
}

/// Resolve which TOML config file to use.
fn resolve_toml_path() -> Option<PathBuf> {
    // 1. Explicit env var
    if let Some(explicit) = std::env::var("TELEPI_CONFIG").ok().filter(|s| !s.trim().is_empty()) {
        let p = paths::resolve_from_cwd(&explicit);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. ./telepi.toml in cwd
    let cwd_toml = std::env::current_dir()
        .unwrap_or_default()
        .join("telepi.toml");
    if cwd_toml.exists() {
        return Some(cwd_toml);
    }

    // 3. ~/.pi/telepi/config.toml
    let default = paths::default_config_path();
    if default.exists() {
        return Some(default);
    }

    None
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Read an optional environment variable, returning None for empty/whitespace.
fn env_override(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parse comma-separated Telegram user IDs.
fn parse_allowed_user_ids(raw: &str) -> Result<Vec<u64>> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<u64>().map_err(|_| {
                TelePiError::InvalidConfig(format!(
                    "invalid Telegram user id: {s}"
                ))
            })
        })
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

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

    #[test]
    fn test_toml_parse() {
        let toml_str = r#"
[telegram]
bot_token = "test-token"
allowed_user_ids = [123, 456]

[pi]
model = "test-model"
tool_verbosity = "all"
"#;
        let config: TomlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.telegram.bot_token.as_deref(), Some("test-token"));
        assert_eq!(config.telegram.allowed_user_ids, vec![123, 456]);
        assert_eq!(config.pi.model.as_deref(), Some("test-model"));
        assert_eq!(config.pi.tool_verbosity, Some(ToolVerbosity::All));
    }

    #[test]
    fn test_toml_parse_defaults() {
        let toml_str = r#"
[telegram]
bot_token = "test-token"
"#;
        let config: TomlConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.telegram.bot_token.as_deref(), Some("test-token"));
        assert!(config.telegram.allowed_user_ids.is_empty());
        assert!(config.pi.model.is_none());
        assert!(config.pi.tool_verbosity.is_none());
        assert!(config.prompt_inbox.dir.is_none());
    }

    #[test]
    fn test_toml_parse_empty() {
        let config: TomlConfig = toml::from_str("").unwrap();
        assert!(config.telegram.bot_token.is_none());
        assert!(config.telegram.allowed_user_ids.is_empty());
    }
}
