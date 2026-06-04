# Modules

## Quick Reference

| Module | Path | Files | Responsibility |
|---|---|---|---|
| bot | `src/bot/` | 5 + 5 commands | Telegram bot handler, commands, state, transport |
| pi | `src/pi/` | 4 | Pi agent session management (trait + CLI impl + registry) |
| install | `src/install/` | 4 | Service installation for launchd/systemd, status detection |
| voice | `src/voice/` | 1 | Voice message transcription (3 backends) |
| cli | `src/cli.rs` | 1 | CLI argument parsing (clap) |
| config | `src/config.rs` | 1 | Config loading from env/.env, validation |
| error | `src/error.rs` | 1 | Error types (thiserror), friendly display |
| format | `src/format.rs` | 1 | HTML escaping for Telegram |
| paths | `src/paths.rs` | 1 | Path resolution, platform-aware defaults |
| entry | `src/main.rs` | 1 | tokio entrypoint, command dispatch |
| lib | `src/lib.rs` | 1 | Module declarations |

---

## `bot/` — Telegram Bot Layer

### Files

| File | Lines | Role |
|---|---|---|
| `mod.rs` | ~65 | `pub async fn run(config)` — builds teloxide Dispatcher, registers command filter chain |
| `handler.rs` | ~467 | Message endpoints: `text_handler`, `voice_handler`, `photo_handler`, `abort_handler`, `retry_handler`, `process_prompt` (streaming) |
| `state.rs` | ~107 | `BotChatState` — per-chat status tracking (Idle/Processing/Switching/Transcribing), last prompt storage |
| `transport.rs` | ~82 | `send_text`, `edit_text`, `send_typing` — Telegram API wrappers with 4096-char chunking |
| `keyboard.rs` | ~71 | `paginate_keyboard` — generic paginated inline keyboard builder |
| `commands/mod.rs` | ~113 | `Command` enum (BotCommands derive), `dispatch()` router, `register_menu()`, `send_welcome()` |
| `commands/context.rs` | ~33 | `/context` — shows session stats (tokens, messages) |
| `commands/model.rs` | ~30 | `/model` — shows current AI model |
| `commands/sessions.rs` | ~83 | `/new` (destroy + recreate), `/sessions` (list), `/handback` (resume-in-terminal instructions) |
| `commands/tree.rs` | ~24 | `/tree`, `/branch`, `/label` — all stubs |
| `commands/basic.rs` | ~41 | `cmd_start`, `cmd_help` — **not compiled** (file exists but `pub mod basic` is not declared in `commands/mod.rs`) |

### Public API

```rust
// mod.rs
pub async fn run(config: TelePiConfig) -> anyhow::Result<()>

// handler.rs
pub struct HandlerState { config, sessions, chat_state }
pub async fn text_handler(bot, msg, state) -> ResponseResult<()>
pub async fn voice_handler(bot, msg, state) -> ResponseResult<()>
pub async fn photo_handler(bot, msg, state) -> ResponseResult<()>
pub async fn abort_handler(bot, msg, state) -> ResponseResult<()>
pub async fn retry_handler(bot, msg, state) -> ResponseResult<()>

// state.rs
pub type ChatKey = String;  // "{chat_id}" or "{chat_id}::{thread_id}"
pub enum ChatStatus { Idle, Processing, Switching, Transcribing }
pub struct BotChatState  // Clone, Arc<Mutex<>> interior
pub fn chat_key(chat_id, thread_id) -> ChatKey
pub fn chat_key_to_context(key) -> SessionContext

// transport.rs
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096
pub async fn send_text(bot, chat_id, reply_to, text) -> Result<Message>
pub async fn edit_text(bot, chat_id, message_id, text) -> Result<()>
pub async fn send_typing(bot, chat_id) -> Result<()>

// keyboard.rs
pub const KEYBOARD_PAGE_SIZE: usize = 6
pub struct KeyboardItem { label, callback_data }
pub fn paginate_keyboard(items, page, filter_prefix) -> (InlineKeyboardMarkup, usize)

// commands/mod.rs
pub enum Command { Start, Help, New, Sessions, Handback, Abort, Retry, Model, Tree, Context }
pub async fn register_menu(bot) -> Result<()>
pub async fn dispatch(bot, msg, cmd, state) -> ResponseResult<()>
```

### Internal Dependencies

- `handler.rs` → `state`, `transport`, `config`, `format`, `pi::registry`, `pi::session::PiEvent`
- `state.rs` → `pi::session::SessionContext`
- `commands/*.rs` → `handler::HandlerState`, `state`, `pi::registry`
- `transport.rs`, `keyboard.rs` → teloxide only (no internal deps)

### Patterns

- **dptree filter chain**: command → voice → photo → text (priority order)
- **Busy guard**: every handler checks `state.is_busy(key)` before processing
- **Optimistic edit**: send "🤔 Thinking...", then stream-edit with actual response
- **Streaming prompt**: `process_prompt` uses `prompt_streaming` + `mpsc::channel<PiEvent>` to receive text deltas, tool events, and turn-end; a spawned task applies debounced edits (1.5s interval) to the Telegram message; tool call start/end shows inline `🔧 <i>tool_name</i>...` indicators

---

## `pi/` — Pi Agent Session Layer

### Files

| File | Lines | Role |
|---|---|---|
| `mod.rs` | 3 | Re-exports |
| `session.rs` | ~117 | `PiSession` trait + data types (`SessionContext`, `SessionInfo`, `PromptResponse`, `ToolCallRecord`, `SessionStats`, `PiEvent`) |
| `registry.rs` | ~90 | `SessionRegistry` — HashMap-based session store, `get_or_create` with double-checked locking |
| `cli_session.rs` | ~489 | `CliSession` — `PiSession` impl backed by `pi --mode json --print` CLI subprocess with streaming JSON event parsing |

### Public API

```rust
// session.rs
pub struct SessionContext { chat_id: i64, message_thread_id: Option<i32> }
pub struct SessionInfo { session_id, session_path, workspace, model, session_name }
pub struct PromptResponse { text: String, tool_calls: Vec<ToolCallRecord> }
pub struct ToolCallRecord { tool_name, tool_call_id, output, is_error }
pub struct SessionStats { session_id, total_messages, tokens_in, tokens_out, cost }

pub enum PiEvent {
    ThinkingDelta { delta },
    TextDelta { delta },
    ToolStart { tool_name, tool_call_id },
    ToolOutput { tool_call_id, output, is_error },
    ToolEnd { tool_call_id },
    Usage { tokens_in, tokens_out, cost, model },
    TurnEnd { text },
    Error { message },
}

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

// registry.rs
pub struct SessionRegistry  // Clone, Arc<RwLock<>>
pub async fn get_or_create(&self, ctx: &SessionContext) -> Result<Arc<dyn PiSession>>
pub async fn remove(&self, ctx: &SessionContext)
pub async fn list(&self) -> Vec<SessionInfo>

// cli_session.rs
pub struct CliSession
pub async fn create(config, ctx, bootstrap_session_path) -> Result<Self>
pub fn pi_cli_available() -> bool  // which::which("pi")
```

### Key Details

- **Bootstrap path**: `PI_SESSION_PATH` from config is consumed by the first `get_or_create` call (`Option::take`). All subsequent chats get fresh sessions.
- **Streaming JSON protocol**: `CliSession` spawns `pi --mode json --print <text>` and reads stdout line-by-line. Each line is parsed as a `JsonEvent` (tagged enum with serde). `MessageUpdate` events carry `AssistantMessageEvent` variants (`ThinkingDelta`, `TextDelta`, `ToolStart`, `ToolEnd`, etc.) which are translated to `PiEvent`s and sent through the `mpsc::Sender`.
- **JSON event types** (internal to `cli_session.rs`): `JsonEvent` (Session, AgentStart, AgentEnd, TurnStart, TurnEnd, MessageStart, MessageEnd, MessageUpdate, Unknown), `AssistantMessageEvent` (ThinkingStart/Delta/End, TextStart/Delta/End, ToolStart/Update/End, Unknown), `JsonMessage`, `JsonUsage`, `JsonCost`, `ToolCallInfo`.
- **Abort support**: `CliSession` stores `running_child: Arc<Mutex<Option<Child>>>`. `abort()` sends `SIGTERM` to the child PID (unix only).
- **Session storage**: data dir per platform (`dirs::data_dir()/telepi/sessions/<uuid>`), or bootstrap path.
- **Not yet implemented**: `stats()` returns zeros (TODO: parse session JSONL), `set_model()` is a no-op (TODO: persist model selection), `prompt_with_images` uses `@file` CLI syntax.

---

## `install/` — Service Installation

### Files

| File | Lines | Role |
|---|---|---|
| `mod.rs` | ~74 | `get_status()` — aggregates config, service, and extension status |
| `platform.rs` | ~53 | `Platform` enum (MacOs/Linux), `detect_platform()`, status structs |
| `launchd.rs` | ~62 | `build_plist()` — generates macOS launchd XML, `installed_plist_path()` |
| `systemd.rs` | ~43 | `build_unit()` — generates systemd unit file, `installed_unit_path()` |

### Public API

```rust
pub async fn get_status() -> TelePiStatus  // mod.rs
pub enum Platform { MacOs, Linux }         // platform.rs
pub fn detect_platform() -> Option<Platform>
pub struct ServiceStatus { installed, running, platform, unit_path }
pub struct ExtensionStatus { installed, path, method }
pub struct TelePiStatus { version, config_path, service, extension }
pub fn build_plist(bin, config, log_dir) -> String   // launchd.rs
pub fn installed_plist_path() -> PathBuf
pub fn build_unit(bin, config, log_dir) -> String    // systemd.rs
pub fn installed_unit_path() -> PathBuf
```

### Key Details

- **Extension detection**: looks for `~/.pi/agent/extensions/telepi-handoff.ts`
- **Service running check**: TODO (always returns `running: false`)
- **Used by**: `Commands::Status` in main.rs

---

## `voice/` — Voice Transcription

### Files

| File | Lines | Role |
|---|---|---|
| `mod.rs` | ~131 | Backend detection, `transcribe()` dispatcher, OpenAI Whisper implementation |

### Public API

```rust
pub struct TranscriptionResult { text, backend, duration_ms }
pub enum VoiceBackend { Parakeet, SherpaOnnx, OpenAi }
pub fn available_backends() -> Vec<VoiceBackend>
pub async fn transcribe(file_path: &Path) -> Result<TranscriptionResult>
```

### Key Details

- **Priority order**: Parakeet (macOS Apple Silicon) > Sherpa-ONNX (cross-platform) > OpenAI Whisper (cloud)
- **Only OpenAI is implemented**. Parakeet and Sherpa-ONNX return `Err(not yet implemented)`.
- **Wired up**: `voice_handler` in `bot/handler.rs` calls `crate::voice::transcribe` for voice/audio messages, then feeds the transcript as a prompt.

---

## `cli.rs` — CLI Definition

| Item | Type | Details |
|---|---|---|
| `Cli` | struct | `#[command(name = "telepi")]`, optional `Commands` subcommand |
| `Commands::Start` | variant | Default. Starts bot polling. |
| `Commands::Setup` | variant | Placeholder. Fields: `bot_token`, `user_ids`, `workspace` (all optional) |
| `Commands::Status` | variant | Calls `install::get_status()`, prints diagnostics |

---

## `config.rs` — Configuration

### Key Types

```rust
pub struct TelePiConfig {
    pub telegram_bot_token: String,
    pub telegram_allowed_user_ids: Vec<u64>,
    pub workspace: PathBuf,
    pub tool_verbosity: ToolVerbosity,       // All | Summary | ErrorsOnly | None
    pub prompt_inbox_dir: Option<PathBuf>,
    pub prompt_inbox_interval_ms: u64,
    pub openai_api_key: Option<String>,
    pub sherpa_onnx_model_dir: Option<PathBuf>,
    pub sherpa_onnx_num_threads: u32,
    pub pi_session_path: Option<PathBuf>,
    pub pi_model: Option<String>,
    pub config_source: ConfigSource,
}
```

### Config Resolution

1. `TELEPI_CONFIG` env → explicit path
2. `.env` in cwd
3. `~/.config/telepi/.env`

Workspace resolution: `/workspace` (Docker) → `TELEPI_WORKSPACE` → cwd → `.`

### Env Vars

| Var | Required | Default |
|---|---|---|
| `TELEGRAM_BOT_TOKEN` | yes | — |
| `TELEGRAM_ALLOWED_USER_IDS` | yes | — |
| `TELEPI_WORKSPACE` | no | cwd |
| `TOOL_VERBOSITY` | no | `summary` |
| `TELEPI_PROMPT_INBOX_DIR` | no | — |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | no | 60000 |
| `OPENAI_API_KEY` | no | — |
| `SHERPA_ONNX_MODEL_DIR` | no | — |
| `SHERPA_ONNX_NUM_THREADS` | no | 2 |
| `PI_SESSION_PATH` | no | — |
| `PI_MODEL` | no | — |

---

## `error.rs` — Error Handling

```rust
pub enum TelePiError {
    MissingEnv(&'static str),
    InvalidConfig(String),
    Telegram(String),
    PiSession(String),
    PiProcess(String),
    Voice(String),
    Install(String),
    Io(std::io::Error),
    Http(reqwest::Error),
    Serde(serde_json::Error),
    Other(anyhow::Error),
}
pub fn to_friendly_error(err) -> String  // strips internal prefixes
```

All variants are string-wrapped except the three `From` impls (Io, Http, Serde).
`teloxide::RequestError` converts to `Telegram(String)`.

---

## `paths.rs` — Path Utilities

| Function | Returns |
|---|---|
| `home_dir()` | `dirs::home_dir()` |
| `expand_home("~/foo")` | resolved absolute path |
| `resolve_from_cwd(path)` | absolute path |
| `default_config_dir()` | `~/.config/telepi/` |
| `default_config_path()` | `~/.config/telepi/.env` |
| `default_systemd_user_dir()` | `~/.config/systemd/user/` |
| `default_log_dir()` | macOS: `~/Library/Logs/TelePi/`, Linux: `~/.local/state/telepi/logs/` |
| `DOCKER_WORKSPACE_PATH` | `"/workspace"` |

---

## `format.rs` — Formatting

Single function: `escape_html(text) -> String`. Replaces `&`, `<`, `>` for Telegram HTML parse mode.
