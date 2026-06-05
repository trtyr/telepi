# API Reference

Public contract for TelePi: CLI interface, Telegram bot commands, data models, and configuration.

---

## CLI Commands

Entry point: `telepi [COMMAND]`

| Command | Description | Parameters |
|---------|-------------|------------|
| `start` | Start the Telegram bot (default) | — |
| `setup` | Interactive setup wizard | `--bot-token <TOKEN>`, `--user-ids <IDS>`, `--workspace <PATH>` (all optional, non-interactive mode) |
| `status` | Show version, config, and service status | — |

**Defaults:** Running `telepi` with no subcommand is equivalent to `telepi start`.

---

## Telegram Bot Commands

All commands use `/command` syntax, case-insensitive. Commands are auto-registered with the Telegram command menu on startup.

| Command | Description | Behavior |
|---------|-------------|----------|
| `/start` | Welcome message and command list | Shows help text with all available commands |
| `/help` | Alias for `/start` | Same as `/start` |
| `/new` | Create a fresh session | Disposes existing session for this chat, creates a new Pi session, reports workspace path |
| `/sessions` | List all active sessions | Shows session ID and workspace for each active session |
| `/handback` | Resume session in terminal | Prints `PI_SESSION_PATH=<path> pi` command for terminal resumption |
| `/abort` | Cancel running operation | Aborts the current Pi prompt (TODO: not yet implemented) |
| `/retry` | Re-send last prompt | Re-sends the last prompt text from this chat's history |
| `/model` | Show current AI model | Displays the active model name, or "using default" |
| `/context` | Show context window usage | Reports session ID, message count, tokens in/out |
| `/tree` | View conversation tree | Stub — not yet implemented |

### Unauthorized Users

All commands and messages from unauthorized user IDs receive: "⛔ You are not authorized."

### Busy Guard

When a prompt is in-flight, new text messages get: "⏳ Still processing the previous prompt. Use /abort to cancel."

---

## Message Types

| Input | Behavior |
|-------|----------|
| **Text message** | Forwarded as prompt to the chat's Pi session; response streamed back in chunks |
| **Voice note** | Transcribed via configured backend (OpenAI Whisper or Sherpa ONNX), then treated as text prompt |
| **Photo** | Sent with optional caption as multimodal prompt to Pi session |

### Message Splitting

Responses exceeding 4096 characters are split at newline boundaries into multiple messages.

---

## Data Models

### `TelePiConfig`

Fully resolved configuration. Loaded from environment variables with `.env` file fallback.

```rust
pub struct TelePiConfig {
    pub telegram_bot_token: String,             // Required. Bot API token
    pub telegram_allowed_user_ids: Vec<u64>,    // Required. Comma-separated Telegram user IDs
    pub workspace: PathBuf,                     // Working directory for Pi sessions
    pub tool_verbosity: ToolVerbosity,          // Output verbosity level
    pub prompt_inbox_dir: Option<PathBuf>,      // Directory for prompt inbox polling
    pub prompt_inbox_interval_ms: u64,          // Inbox polling interval (default: 60000)
    pub openai_api_key: Option<String>,         // For Whisper voice transcription
    pub sherpa_onnx_model_dir: Option<PathBuf>, // Local ONNX model for transcription
    pub sherpa_onnx_num_threads: u32,           // ONNX inference threads (default: 2)
    pub pi_session_path: Option<PathBuf>,       // Bootstrap session path (consumed once)
    pub pi_model: Option<String>,               // Default AI model override
}
```

**Public method:** `is_allowed_user(user_id: u64) -> bool`

### `ToolVerbosity`

```rust
pub enum ToolVerbosity {
    All,         // Show all tool output
    Summary,     // Show summaries (default)
    ErrorsOnly,  // Show only errors
    None,        // Suppress tool output
}
```

### `SessionContext`

Identifies a unique Telegram conversation → Pi session mapping.

```rust
pub struct SessionContext {
    pub chat_id: i64,
    pub message_thread_id: Option<i32>,
}
// Display: "chat_id" or "chat_id::thread_id"
```

### `SessionInfo`

Read-only metadata about a Pi session.

```rust
pub struct SessionInfo {
    pub session_id: String,
    pub session_path: PathBuf,
    pub workspace: PathBuf,
    pub model: Option<String>,
    pub session_name: Option<String>,
}
```

### `PromptResponse`

Result of a completed prompt execution.

```rust
pub struct PromptResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

### `ToolCallRecord`

```rust
pub struct ToolCallRecord {
    pub tool_name: String,
    pub tool_call_id: String,
    pub output: Option<String>,
    pub is_error: bool,
}
```

### `SessionStats`

```rust
pub struct SessionStats {
    pub session_id: String,
    pub total_messages: usize,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost: f64,
}
```

### `PiEvent` (Streaming)

Events emitted during streaming prompt execution, delivered via `mpsc::Sender<PiEvent>`.

```rust
pub enum PiEvent {
    ThinkingDelta { delta: String },
    TextDelta { delta: String },
    ToolStart { tool_name: String, tool_call_id: String },
    ToolOutput { tool_call_id: String, output: String, is_error: bool },
    ToolEnd { tool_call_id: String },
    Usage { tokens_in: u64, tokens_out: u64, cost: f64, model: String },
    TurnEnd { text: String },
    Error { message: String },
}
```

### `ChatStatus`

```rust
pub enum ChatStatus {
    Idle,
    Processing,
    Switching,
    Transcribing,
}
```

---

## Traits

### `PiSession`

Core abstraction for a Pi coding agent session. All session interactions go through this trait.

```rust
#[async_trait]
pub trait PiSession: Send + Sync {
    fn info(&self) -> SessionInfo;
    async fn stats(&self) -> SessionStats;
    async fn prompt(&self, text: &str) -> Result<PromptResponse>;
    async fn prompt_with_images(&self, text: &str, images: &[PathBuf]) -> Result<PromptResponse>;
    async fn prompt_streaming(&self, text: &str, tx: mpsc::Sender<PiEvent>) -> Result<PromptResponse>;
    async fn abort(&self) -> Result<()>;
    async fn set_model(&self, model: &str) -> Result<()>;
    async fn dispose(&self) -> Result<()>;
}
```

### `SessionRegistry`

Thread-safe per-chat session manager. Clone-safe (wraps `Arc<RwLock<…>>`).

```rust
pub struct SessionRegistry { /* ... */ }

impl SessionRegistry {
    pub fn new(config: Arc<TelePiConfig>) -> Self;
    pub async fn get_or_create(&self, ctx: &SessionContext) -> Result<Arc<dyn PiSession>>;
    pub async fn remove(&self, ctx: &SessionContext);
    pub async fn list(&self) -> Vec<SessionInfo>;
}
```

### `BotChatState`

Thread-safe per-chat busy guard. Clone-safe (wraps `Arc<Mutex<…>>`).

```rust
pub struct BotChatState { /* ... */ }

impl BotChatState {
    pub fn new() -> Self;
    pub async fn status(&self, key: &ChatKey) -> ChatStatus;
    pub async fn is_busy(&self, key: &ChatKey) -> bool;
    pub async fn begin_processing(&self, key: &ChatKey, prompt: &str);
    pub async fn end_processing(&self, key: &ChatKey);
    pub async fn last_prompt(&self, key: &ChatKey) -> Option<String>;
}
```

---

## Configuration Resolution

### Config File

1. `TELEPI_CONFIG` env var → explicit path
2. `.env` in current working directory
3. `~/.pi/telepi/config.toml` (default)

### Workspace

1. `/workspace` (Docker, if non-empty)
2. `TELEPI_WORKSPACE` env var
3. Current working directory

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TELEGRAM_BOT_TOKEN` | Yes | — | Telegram Bot API token |
| `TELEGRAM_ALLOWED_USER_IDS` | Yes | — | Comma-separated authorized user IDs |
| `TELEPI_WORKSPACE` | No | cwd | Pi session working directory |
| `TELEPI_CONFIG` | No | — | Explicit `.env` file path |
| `TOOL_VERBOSITY` | No | `summary` | `all` / `summary` / `errors-only` / `none` |
| `TELEPI_PROMPT_INBOX_DIR` | No | — | Prompt inbox polling directory |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | No | `60000` | Inbox polling interval (ms) |
| `OPENAI_API_KEY` | No | — | Whisper voice transcription |
| `SHERPA_ONNX_MODEL_DIR` | No | — | Local ONNX model path |
| `SHERPA_ONNX_NUM_THREADS` | No | `2` | ONNX inference threads |
| `PI_SESSION_PATH` | No | — | Bootstrap session (consumed once) |
| `PI_MODEL` | No | — | Default AI model |

---

## Error Types

```rust
pub enum TelePiError {
    MissingEnv(&'static str),     // Required env var not set
    InvalidConfig(String),        // Config parse error
    Telegram(String),             // Telegram API error
    PiSession(String),            // Session management error
    PiProcess(String),            // Pi CLI subprocess error
    Voice(String),                // Voice transcription error
    Install(String),              // Service installation error
    Io(std::io::Error),           // I/O error
    Http(reqwest::Error),         // HTTP client error
    Serde(serde_json::Error),     // JSON serialization error
    Other(anyhow::Error),         // Catch-all
}
```

**User-facing errors** are sanitized via `to_friendly_error()` which strips internal details and returns human-readable messages.

---

## Protocol Details

### Session Lifecycle

1. First message in a chat triggers `SessionRegistry::get_or_create()` → spawns `CliSession` subprocess
2. `PI_SESSION_PATH` is consumed by the first session creation, then ignored
3. `/new` disposes the current session and creates a fresh one
4. Sessions are keyed by `(chat_id, optional_thread_id)` — forum topics get independent sessions

### Busy Guard Flow

```
text received → is_busy? → yes → "Still processing…"
                        → no  → begin_processing → process_prompt → end_processing
```

### Streaming

Prompt responses are streamed via `PiEvent` channel. The bot handler forwards `TextDelta` events as message edits for real-time output.
