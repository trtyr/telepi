use std::path::PathBuf;
use tokio::sync::mpsc;

/// Identifies a Telegram chat context that owns a Pi session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionContext {
    pub chat_id: i64,
    pub message_thread_id: Option<i32>,
}

impl std::fmt::Display for SessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.message_thread_id {
            Some(tid) => write!(f, "{}::{}", self.chat_id, tid),
            None => write!(f, "{}", self.chat_id),
        }
    }
}

/// Metadata about a Pi session.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub session_path: PathBuf,
    pub workspace: PathBuf,
    pub model: Option<String>,
    pub session_name: Option<String>,
}

/// A response from a Pi prompt.
#[derive(Debug, Clone)]
pub struct PromptResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
}

/// Record of a tool call during prompt execution.
#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub tool_call_id: String,
    pub output: Option<String>,
    pub is_error: bool,
}

/// Session statistics.
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub total_messages: usize,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost: f64,
}

/// Events emitted during streaming prompt execution.
#[derive(Debug, Clone)]
pub enum PiEvent {
    /// Thinking/reasoning content (streaming delta).
    ThinkingDelta { delta: String },
    /// Text content (streaming delta).
    TextDelta { delta: String },
    /// A tool call started.
    ToolStart { tool_name: String, tool_call_id: String },
    /// Tool call output.
    ToolOutput { tool_call_id: String, output: String, is_error: bool },
    /// A tool call ended.
    ToolEnd { tool_call_id: String },
    /// Usage statistics (emitted at end).
    Usage { tokens_in: u64, tokens_out: u64, cost: f64, model: String },
    /// Turn ended with the final accumulated text.
    TurnEnd { text: String },
    /// Error during execution.
    Error { message: String },
}

/// Trait abstracting a Pi coding agent session.
///
/// Implementations can use the Pi CLI as a subprocess, or in the future,
/// implement the Pi agent protocol directly.
#[async_trait::async_trait]
pub trait PiSession: Send + Sync {
    /// Get session metadata.
    fn info(&self) -> SessionInfo;

    /// Get session statistics.
    async fn stats(&self) -> SessionStats;

    /// Send a text prompt and get a response (wait for completion).
    async fn prompt(&self, text: &str) -> crate::error::Result<PromptResponse>;

    /// Send a prompt with images.
    async fn prompt_with_images(
        &self,
        text: &str,
        images: &[PathBuf],
    ) -> crate::error::Result<PromptResponse>;

    /// Send a prompt and stream events via channel.
    ///
    /// The implementation sends `PiEvent`s through `tx` as they arrive.
    /// Returns the final `PromptResponse` after the turn completes.
    async fn prompt_streaming(
        &self,
        text: &str,
        tx: mpsc::Sender<PiEvent>,
    ) -> crate::error::Result<PromptResponse>;

    /// Abort the current running prompt.
    async fn abort(&self) -> crate::error::Result<()>;

    /// Switch the AI model.
    async fn set_model(&self, model: &str) -> crate::error::Result<()>;

    /// Dispose of the session and clean up resources.
    async fn dispose(&self) -> crate::error::Result<()>;
}
