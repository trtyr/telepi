use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

use crate::config::TelePiConfig;
use crate::error::{Result, TelePiError};
use crate::pi::session::*;

/// Pi session backed by the `pi` CLI subprocess.
///
/// Uses `pi --mode json --print` for structured streaming output.
/// Each line from stdout is a JSON event that gets parsed into `PiEvent`s.
#[allow(dead_code)]
pub struct CliSession {
    config: Arc<TelePiConfig>,
    ctx: SessionContext,
    session_path: PathBuf,
    workspace: PathBuf,
    session_id: String,
    model: Mutex<Option<String>>,
    /// Handle to the currently running child process (for abort).
    running_child: Arc<Mutex<Option<Child>>>,
}

impl std::fmt::Debug for CliSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliSession")
            .field("session_id", &self.session_id)
            .field("workspace", &self.workspace)
            .finish()
    }
}

// --- JSON event types from `pi --mode json` ---

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum JsonEvent {
    #[serde(rename = "session")]
    Session { id: Option<String> },
    #[serde(rename = "agent_start")]
    AgentStart,
    #[serde(rename = "agent_end")]
    AgentEnd,
    #[serde(rename = "turn_start")]
    TurnStart,
    #[serde(rename = "turn_end")]
    TurnEnd { message: Option<JsonMessage> },
    #[serde(rename = "message_start")]
    MessageStart { message: JsonMessage },
    #[serde(rename = "message_end")]
    MessageEnd { message: JsonMessage },
    #[serde(rename = "message_update")]
    MessageUpdate { #[serde(rename = "assistantMessageEvent")] assistant_message_event: Option<AssistantMessageEvent> },
    /// Catch-all for unknown events.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonMessage {
    role: Option<String>,
    content: Option<serde_json::Value>,
    usage: Option<JsonUsage>,
    #[serde(rename = "stopReason")]
    stop_reason: Option<String>,
    #[serde(rename = "responseId")]
    response_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct JsonUsage {
    input: Option<u64>,
    output: Option<u64>,
    #[serde(rename = "cacheRead")]
    cache_read: Option<u64>,
    #[serde(rename = "cacheWrite")]
    cache_write: Option<u64>,
    cost: Option<JsonCost>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonCost {
    total: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
enum AssistantMessageEvent {
    #[serde(rename = "thinking_start")]
    ThinkingStart { content_index: Option<u32>, partial: Option<JsonMessage> },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { delta: String, content_index: Option<u32>, partial: Option<JsonMessage> },
    #[serde(rename = "thinking_end")]
    ThinkingEnd { content_index: Option<u32>, content: Option<String> },
    #[serde(rename = "text_start")]
    TextStart { content_index: Option<u32>, partial: Option<JsonMessage> },
    #[serde(rename = "text_delta")]
    TextDelta { delta: String, content_index: Option<u32>, partial: Option<JsonMessage> },
    #[serde(rename = "text_end")]
    TextEnd { content_index: Option<u32>, content: Option<String> },
    #[serde(rename = "tool_start")]
    ToolStart {
        tool_name: Option<String>,
        #[serde(rename = "toolCallId")]
        tool_call_id: Option<String>,
        #[serde(rename = "toolCall")]
        tool_call: Option<ToolCallInfo>,
        partial: Option<JsonMessage>,
    },
    #[serde(rename = "tool_update")]
    ToolUpdate {
        #[serde(rename = "toolCallId")]
        tool_call_id: Option<String>,
        output: Option<String>,
        partial: Option<JsonMessage>,
    },
    #[serde(rename = "tool_end")]
    ToolEnd {
        #[serde(rename = "toolCallId")]
        tool_call_id: Option<String>,
        #[serde(rename = "is_error")]
        is_error: Option<bool>,
        partial: Option<JsonMessage>,
    },
    /// Catch-all for unknown assistant events.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ToolCallInfo {
    name: Option<String>,
    #[serde(rename = "toolCallId")]
    tool_call_id: Option<String>,
}

/// Information about an available AI model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub context_window: String,
    pub max_output: String,
    pub supports_thinking: bool,
    pub supports_images: bool,
}

impl std::fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.provider, self.model)
    }
}

/// Parse the output of `pi --list-models`.
fn parse_model_list(output: &str) -> Vec<ModelInfo> {
    let mut models = Vec::new();
    for line in output.lines().skip(1) {
        // Skip header line
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Split by whitespace — the format is fixed-width columns
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 6 {
            models.push(ModelInfo {
                provider: parts[0].to_string(),
                model: parts[1].to_string(),
                context_window: parts[2].to_string(),
                max_output: parts[3].to_string(),
                supports_thinking: parts[4] == "yes",
                supports_images: parts[5] == "yes",
            });
        }
    }
    models
}
impl CliSession {
    /// Create a new CLI-backed Pi session.
    pub async fn create(
        config: Arc<TelePiConfig>,
        ctx: SessionContext,
        bootstrap_session_path: Option<PathBuf>,
    ) -> Result<Self> {
        let workspace = config.workspace.clone();
        let session_id = uuid::Uuid::new_v4().to_string();

        // If we have a bootstrap session path, use it; otherwise create a new one
        let session_path = match bootstrap_session_path {
            Some(p) if p.exists() => {
                info!(path = %p.display(), "using bootstrap session");
                p
            }
            _ => {
                // Create a new session directory
                let session_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("telepi")
                    .join("sessions")
                    .join(&session_id);
                tokio::fs::create_dir_all(&session_dir).await?;
                session_dir
            }
        };

        Ok(Self {
            config,
            ctx,
            session_path,
            workspace,
            session_id,
            model: Mutex::new(None),
            running_child: Arc::new(Mutex::new(None)),
        })
    }

    /// Check if the `pi` CLI is available on PATH.
    pub fn pi_cli_available() -> bool {
        which::which("pi").is_ok()
    }

    /// List available models from `pi --list-models`.
    pub async fn list_models() -> Result<Vec<ModelInfo>> {
        let output = tokio::process::Command::new("pi")
            .arg("--list-models")
            .output()
            .await
            .map_err(|e| TelePiError::PiProcess(format!("failed to run pi --list-models: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TelePiError::PiProcess(format!("pi --list-models failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let models = parse_model_list(&stdout);
        Ok(models)
    }
    /// Execute a streaming prompt, parsing JSON events line by line.
    async fn execute_streaming(
        &self,
        args: Vec<String>,
        tx: &mpsc::Sender<PiEvent>,
    ) -> Result<PromptResponse> {
        let pi_bin = which::which("pi")
            .map_err(|_| TelePiError::PiProcess("`pi` CLI not found on PATH".into()))?;

        info!(
            session_id = %self.session_id,
            args = ?args,
            "spawning pi CLI in streaming mode"
        );

        let mut cmd = Command::new(&pi_bin);
        cmd.args(&args)
            .current_dir(&self.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PI_SESSION_PATH", &self.session_path);

        let model_guard = self.model.lock().await.clone();
        if let Some(ref m) = model_guard {
            cmd.env("PI_MODEL", m);
        }

        let child = cmd.spawn()
            .map_err(|e| TelePiError::PiProcess(format!("failed to spawn pi: {e}")))?;

        // Store the child PID for abort support
        let _child_id = child.id();
        {
            let mut running = self.running_child.lock().await;
            *running = Some(child);
        }

        // Take stdout for reading
        let stdout = {
            let mut running = self.running_child.lock().await;
            running.as_mut().and_then(|c| c.stdout.take())
        };
        let _stderr = {
            let mut running = self.running_child.lock().await;
            running.as_mut().and_then(|c| c.stderr.take())
        };

        let stdout = stdout.ok_or_else(|| TelePiError::PiProcess("no stdout from pi".into()))?;

        // Wrap the entire streaming + wait in a 10-minute timeout
        let timeout_result = tokio::time::timeout(Duration::from_secs(600), async {
            // Read JSON events line by line
            let mut accumulated_text = String::new();
            let mut tokens_in: u64 = 0;
            let mut tokens_out: u64 = 0;
            let mut cost: f64 = 0.0;
            let model_name = String::new();

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.map_err(|e| {
            TelePiError::PiProcess(format!("failed to read pi output: {e}"))
        })? {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the JSON event
            match serde_json::from_str::<JsonEvent>(&line) {
                Ok(event) => {
                    match event {
                        JsonEvent::MessageUpdate { assistant_message_event: Some(ame) } => {
                            match ame {
                                AssistantMessageEvent::ThinkingDelta { delta, .. } => {
                                    tx.send(PiEvent::ThinkingDelta { delta }).await.ok();
                                }
                                AssistantMessageEvent::TextDelta { delta, .. } => {
                                    accumulated_text.push_str(&delta);
                                    tx.send(PiEvent::TextDelta { delta }).await.ok();
                                }
                                AssistantMessageEvent::ToolStart { tool_name, tool_call_id, .. } => {
                                    let name = tool_name.unwrap_or_else(|| "unknown".into());
                                    let id = tool_call_id.unwrap_or_else(|| "unknown".into());
                                    tx.send(PiEvent::ToolStart {
                                        tool_name: name,
                                        tool_call_id: id,
                                    }).await.ok();
                                }
                                AssistantMessageEvent::ToolEnd { tool_call_id, is_error: _, .. } => {
                                    let id = tool_call_id.unwrap_or_else(|| "unknown".into());
                                    tx.send(PiEvent::ToolEnd {
                                        tool_call_id: id,
                                    }).await.ok();
                                }
                                _ => {
                                    // thinking_start, thinking_end, text_start, text_end,
                                    // tool_update, unknown — skip for now
                                }
                            }
                        }
                        JsonEvent::MessageUpdate { assistant_message_event: None } => {
                            // No assistant event — ignore
                        }
                        JsonEvent::MessageEnd { message } => {
                            // Extract usage info from assistant message end
                            if let Some(usage) = &message.usage {
                                tokens_in = usage.input.unwrap_or(0);
                                tokens_out = usage.output.unwrap_or(0);
                                cost = usage.cost.as_ref().and_then(|c| c.total).unwrap_or(0.0);
                            }
                        }
                        JsonEvent::TurnEnd { message } => {
                            // Extract final usage if available
                            if let Some(msg) = &message {
                                if let Some(usage) = &msg.usage {
                                    tokens_in = usage.input.unwrap_or(0);
                                    tokens_out = usage.output.unwrap_or(0);
                                    cost = usage.cost.as_ref().and_then(|c| c.total).unwrap_or(0.0);
                                }
                            }
                        }
                        JsonEvent::Session { .. }
                        | JsonEvent::AgentStart
                        | JsonEvent::AgentEnd
                        | JsonEvent::MessageStart { .. }
                        | JsonEvent::TurnStart => {
                            // Lifecycle events — ignore for now
                        }
                        JsonEvent::Unknown => {
                            // Unknown event type — ignore
                        }
                    }
                }
                Err(_) => {
                    // Not a JSON event line — might be plain text output (fallback)
                    // Only accumulate if we haven't gotten any text_delta events
                    if accumulated_text.is_empty() {
                        if !accumulated_text.is_empty() {
                            accumulated_text.push('\n');
                        }
                        accumulated_text.push_str(&line);
                    }
                }
            }
        }

        // Wait for child to exit
        let exit_status = {
            let mut running = self.running_child.lock().await;
            if let Some(ref mut child) = *running {
                child.wait().await.map_err(|e| {
                    TelePiError::PiProcess(format!("pi process error: {e}"))
                })?
            } else {
                return Err(TelePiError::PiProcess("child process disappeared".into()));
            }
        };

        // Clear the running child
        {
            let mut running = self.running_child.lock().await;
            *running = None;
        }

        // If the process failed and we have no output, report error
        if !exit_status.success() && accumulated_text.is_empty() {
            error!("pi process exited with error");
            return Err(TelePiError::PiProcess(
                "pi process failed — check pi CLI output".into()
            ));
        }

        // Send usage event
        tx.send(PiEvent::Usage {
            tokens_in,
            tokens_out,
            cost,
            model: model_name,
        }).await.ok();

        // Send turn end event
        tx.send(PiEvent::TurnEnd {
            text: accumulated_text.clone(),
        }).await.ok();

        info!(
            text_len = accumulated_text.len(),
            tokens_in,
            tokens_out,
            "streaming complete"
        );

            Ok(PromptResponse {
                text: accumulated_text,
                tool_calls: vec![],
            })
        }).await;

        match timeout_result {
            Ok(result) => result,
            Err(_elapsed) => {
                warn!(
                    session_id = %self.session_id,
                    "pi process timed out after 600s, killing"
                );
                let mut running = self.running_child.lock().await;
                if let Some(ref mut child) = *running {
                    child.kill().await.ok();
                }
                *running = None;
                Err(TelePiError::PiProcess(
                    "pi process timed out after 10 minutes".into()
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl PiSession for CliSession {
    fn info(&self) -> SessionInfo {
        SessionInfo {
            session_id: self.session_id.clone(),
            session_path: self.session_path.clone(),
            workspace: self.workspace.clone(),
            model: self.model.try_lock().ok().and_then(|g| g.clone()),
            session_name: None,
        }
    }

    async fn stats(&self) -> SessionStats {
        // TODO: Parse session JSONL file for actual stats
        SessionStats {
            session_id: self.session_id.clone(),
            total_messages: 0,
            tokens_in: 0,
            tokens_out: 0,
            cost: 0.0,
        }
    }

    async fn prompt(&self, text: &str) -> Result<PromptResponse> {
        let (tx, _rx) = mpsc::channel(256);
        let args = vec![
            "--mode".to_string(), "json".to_string(),
            "--print".to_string(),
            "--session".to_string(), self.session_path.display().to_string(),
            "--continue".to_string(),
            text.to_string(),
        ];
        self.execute_streaming(args, &tx).await
    }

    async fn prompt_with_images(&self, text: &str, images: &[PathBuf]) -> Result<PromptResponse> {
        if images.is_empty() {
            return self.prompt(text).await;
        }

        let (tx, _rx) = mpsc::channel(256);

        // Build args with @file syntax for images
        let mut args = vec![
            "--mode".to_string(), "json".to_string(),
            "--print".to_string(),
            "--session".to_string(), self.session_path.display().to_string(),
            "--continue".to_string(),
        ];
        for img in images {
            args.push(format!("@{}", img.display()));
        }
        args.push(text.to_string());

        self.execute_streaming(args, &tx).await
    }

    async fn prompt_streaming(
        &self,
        text: &str,
        tx: mpsc::Sender<PiEvent>,
    ) -> Result<PromptResponse> {
        let args = vec![
            "--mode".to_string(), "json".to_string(),
            "--print".to_string(),
            "--session".to_string(), self.session_path.display().to_string(),
            "--continue".to_string(),
            text.to_string(),
        ];
        self.execute_streaming(args, &tx).await
    }

    async fn abort(&self) -> Result<()> {
        let mut running = self.running_child.lock().await;
        if let Some(ref mut child) = *running {
            if let Some(pid) = child.id() {
                info!(pid, "sending SIGTERM to pi process");
                // Send SIGTERM first, then SIGKILL if needed
                #[cfg(unix)]
                {
                    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                }
            }
        } else {
            warn!("abort called but no running process");
        }
        Ok(())
    }

    async fn set_model(&self, model: &str) -> Result<()> {
        let mut guard = self.model.lock().await;
        *guard = Some(model.to_string());
        info!(model = model, "model set");
        Ok(())
    }

    async fn dispose(&self) -> Result<()> {
        info!(session_id = %self.session_id, "disposing session");
        Ok(())
    }
}
