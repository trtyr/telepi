use thiserror::Error;

/// Top-level error type for TelePi.
#[derive(Error, Debug)]
pub enum TelePiError {
    #[error("missing required environment variable: {0}")]
    MissingEnv(&'static str),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("telegram error: {0}")]
    Telegram(String),

    #[error("pi session error: {0}")]
    PiSession(String),

    #[error("pi process error: {0}")]
    PiProcess(String),

    #[error("voice transcription error: {0}")]
    Voice(String),

    #[error("install error: {0}")]
    Install(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("reqwest error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl From<teloxide::RequestError> for TelePiError {
    fn from(err: teloxide::RequestError) -> Self {
        TelePiError::Telegram(format!("{err}"))
    }
}

pub type Result<T> = std::result::Result<T, TelePiError>;

/// Strip internal prefixes from error messages before showing to Telegram users.
pub fn to_friendly_error(err: &TelePiError) -> String {
    match err {
        TelePiError::MissingEnv(name) => {
            format!("Missing required setting: `{name}`. Check your `.env` file.")
        }
        TelePiError::InvalidConfig(msg) => format!("Configuration error: {msg}"),
        TelePiError::Telegram(msg) => format!("Telegram error: {msg}"),
        TelePiError::PiSession(msg) => format!("Session error: {msg}"),
        TelePiError::PiProcess(msg) => format!("Pi process error: {msg}"),
        TelePiError::Voice(msg) => format!("Voice error: {msg}"),
        TelePiError::Install(msg) => format!("Install error: {msg}"),
        TelePiError::Io(e) => format!("IO error: {e}"),
        TelePiError::Http(e) => format!("Network error: {e}"),
        TelePiError::Serde(e) => format!("Data format error: {e}"),
        TelePiError::Other(e) => format!("{e}"),
    }
}
