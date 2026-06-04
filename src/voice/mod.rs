use std::path::Path;

use crate::error::{Result, TelePiError};

/// Result of a voice transcription.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub backend: VoiceBackend,
    pub duration_ms: u64,
}

/// Available voice transcription backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceBackend {
    /// Parakeet CoreML — Apple Silicon only
    Parakeet,
    /// Sherpa-ONNX — cross-platform
    SherpaOnnx,
    /// OpenAI Whisper — cloud
    OpenAi,
}

impl std::fmt::Display for VoiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parakeet => write!(f, "parakeet"),
            Self::SherpaOnnx => write!(f, "sherpa-onnx"),
            Self::OpenAi => write!(f, "openai"),
        }
    }
}

/// Check which voice backends are available on this system.
pub fn available_backends() -> Vec<VoiceBackend> {
    let mut backends = Vec::new();

    // Parakeet CoreML — macOS Apple Silicon only
    #[cfg(target_os = "macos")]
    {
        if which::which("parakeet").is_ok() {
            backends.push(VoiceBackend::Parakeet);
        }
    }

    // Sherpa-ONNX — cross-platform
    if which::which("sherpa-onnx").is_ok() || std::env::var("SHERPA_ONNX_MODEL_DIR").is_ok() {
        backends.push(VoiceBackend::SherpaOnnx);
    }

    // OpenAI Whisper — cloud, needs API key
    if std::env::var("OPENAI_API_KEY").is_ok() {
        backends.push(VoiceBackend::OpenAi);
    }

    backends
}

/// Transcribe an audio file using the best available backend.
pub async fn transcribe(file_path: &Path) -> Result<TranscriptionResult> {
    let backends = available_backends();
    let backend = backends.first().copied().ok_or_else(|| {
        TelePiError::Voice("no voice transcription backend available".into())
    })?;

    match backend {
        VoiceBackend::Parakeet => transcribe_parakeet(file_path).await,
        VoiceBackend::SherpaOnnx => transcribe_sherpa(file_path).await,
        VoiceBackend::OpenAi => transcribe_openai(file_path).await,
    }
}

/// Transcribe using Parakeet CoreML.
async fn transcribe_parakeet(_file_path: &Path) -> Result<TranscriptionResult> {
    // TODO: Implement Parakeet CoreML transcription
    Err(TelePiError::Voice("Parakeet transcription not yet implemented".into()))
}

/// Transcribe using Sherpa-ONNX.
async fn transcribe_sherpa(_file_path: &Path) -> Result<TranscriptionResult> {
    // TODO: Implement Sherpa-ONNX transcription
    Err(TelePiError::Voice("Sherpa-ONNX transcription not yet implemented".into()))
}

/// Transcribe using OpenAI Whisper API.
async fn transcribe_openai(file_path: &Path) -> Result<TranscriptionResult> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| TelePiError::Voice("OPENAI_API_KEY not set".into()))?;

    let client = reqwest::Client::new();

    let file_bytes = tokio::fs::read(file_path).await?;
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.ogg");

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name.to_string())
        .mime_str("audio/ogg")
        .map_err(|e| TelePiError::Voice(format!("invalid mime type: {e}")))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", "whisper-1")
        .text("response_format", "text");

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(&api_key)
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(TelePiError::Voice(format!(
            "OpenAI Whisper API error {status}: {body}"
        )));
    }

    let text = response.text().await?;

    Ok(TranscriptionResult {
        text,
        backend: VoiceBackend::OpenAi,
        duration_ms: 0,
    })
}
