# Architecture

## System Overview

Telegram bot that bridges user messages to the **Pi coding agent CLI** (`pi`).
One Rust binary, three runtime modes: `start` (bot), `setup` (interactive config), `status` (diagnostics).
Deployable as a Docker container or system service (launchd/systemd).

```
┌─────────────┐      Telegram API      ┌──────────────────────────────────┐
│  Telegram    │ ◄──── polling ──────►  │  telepi binary                   │
│  User(s)     │                        │                                  │
└─────────────┘                        │  ┌──────────┐  ┌──────────────┐ │
                                        │  │  bot/    │  │  pi/         │ │
  ┌──────────┐                         │  │ handler  │─►│ registry     │ │
  │  inbox/  │──.txt files──►          │  │ commands │  │ session      │ │
  │  dir     │                         │  │ state    │  │ cli_session  │ │
  └──────────┘                         │  │ transport│  │ tree         │ │
                                        │  │ keyboard │  └──────┬───────┘ │
                                        │  │ prompt_  │         │         │
                                        │  │  inbox   │         ▼         │
                                        │  └──────────┘  ┌──────────────┐  │
                                        │               │ pi CLI       │  │
                                        │               │ (json mode)  │  │
                                        │               │ (streaming)  │  │
                                        │               └──────────────┘  │
                                        └──────────────────────────────────┘
```

## Layering Model

```
┌─────────────────────────────────────────────────┐
│  CLI / Entry           main.rs, cli.rs           │  ← clap arg parsing, dispatch
├─────────────────────────────────────────────────┤
│  Bot Layer              bot/                     │  ← teloxide dispatcher, commands
│    handler  — message endpoints, streaming        │
│    commands — /start, /new, /sessions, /tree etc. │
│    state    — per-chat busy/idle tracking        │
│    transport— send/edit/split Telegram messages   │
│    keyboard — paginated inline keyboards         │
│    prompt_inbox — filesystem polling for prompts  │
├─────────────────────────────────────────────────┤
│  Session Layer          pi/                      │  ← PiSession trait, registry
│    registry — HashMap<SessionContext, PiSession>  │
│    session  — trait + data types + PiEvent enum  │
│    cli_session — pi CLI subprocess (json mode)   │
│    tree     — session JSONL tree parsing/renders  │
├─────────────────────────────────────────────────┤
│  Support Layer          config, paths, error,    │  ← shared utilities
│                         format, install, voice   │
├─────────────────────────────────────────────────┤
│  Deployment             Dockerfile,              │  ← container + service
│                         docker-compose.yml,      │
│                         telepi.toml, install/    │
└─────────────────────────────────────────────────┘
```

## Module Dependency Graph

```
main.rs
  └─► cli.rs
  └─► config.rs ──► paths.rs, error.rs
  └─► bot/mod.rs ──► config, pi/registry, bot/*

bot/mod.rs ──► bot/handler, bot/commands, bot/state, bot/prompt_inbox, config, pi/registry
bot/handler.rs ──► bot/state, bot/transport, config, format, pi/registry, pi/session (PiEvent)
bot/commands/mod.rs ──► bot/handler (HandlerState)
bot/commands/sessions.rs ──► bot/handler (HandlerState), bot/state
bot/commands/context.rs ──► bot/handler (HandlerState)
bot/commands/model.rs ──► bot/handler (HandlerState)
bot/commands/tree.rs ──► bot/handler (HandlerState), pi/tree
bot/prompt_inbox.rs ──► bot/handler (HandlerState), bot/state, config
bot/state.rs ──► pi/session (SessionContext)
bot/transport.rs ──► teloxide only
bot/keyboard.rs ──► teloxide only

pi/registry.rs ──► config, error, pi/session, pi/cli_session
pi/cli_session.rs ──► config, error, pi/session
pi/session.rs ──► (pure trait + types, no internal deps)
pi/tree.rs ──► error (serde, std::path)

install/mod.rs ──► paths, install/platform, install/launchd, install/systemd
install/platform.rs ──► (pure types)
install/launchd.rs ──► dirs
install/systemd.rs ──► paths

voice/mod.rs ──► error
format.rs ──► (standalone)
paths.rs ──► dirs
error.rs ──► thiserror, anyhow, teloxide
```

## Data Flow: Telegram Message → Pi Response

### Text / Command Flow (streaming)

```
1. teloxide Dispatcher receives Message
       │
2. Filter chain selects handler branch:
   ┌──────────────────┬───────────────────┬───────────────────┬──────────────┐
   │ command message   │ voice/audio msg   │ photo/document msg│ text msg     │
   │ → commands::dispatch → voice_handler │ → photo_handler   │ → text_handler│
   └──────────────────┴───────────────────┴───────────────────┴──────┬───────┘
                                                                      │
3. text_handler:                                                       ▼
   ├─ auth check (config.is_allowed_user)
   ├─ busy check (state.is_busy)  ──► reject if Processing
   ├─ state.begin_processing(key, prompt)
   ├─ transport.send_typing()
   └─ process_prompt()
       │
4. process_prompt:                                                   ▼
   ├─ state::chat_key_to_context(key) → SessionContext
   ├─ sessions.get_or_create(ctx)
   │      └─ first call: CliSession::create(config, ctx)
   │           └─ creates session dir or uses bootstrap path
   ├─ bot.send_message("🤔 Thinking...")
   ├─ create mpsc::channel::<PiEvent>(256)
   ├─ spawn tokio task for streaming edits (1.5s debounce)
   │      ├─ ThinkingDelta → (suppressed, too noisy for Telegram)
   │      ├─ TextDelta → accumulate + debounced edit ("🔄" indicator)
   │      ├─ ToolStart → show "🔧 <tool_name>..."
   │      ├─ ToolEnd → restore to accumulated text
   │      ├─ TurnEnd → final edit with complete response
   │      └─ Error → append error indicator
   ├─ session.prompt_streaming(prompt, tx)
   │      └─ CliSession: spawns `pi --mode json --print <text>`
   │           └─ stdout parsed line-by-line as JSON events
   │           └─ JsonEvent → PiEvent mapping via mpsc channel
   ├─ wait for edit task (5s timeout)
   └─ format::escape_html(response) → transport::edit_text(final)

5. state.end_processing(key)
```

### Prompt Inbox Flow (filesystem polling)

```
1. bot/mod.rs::run() calls prompt_inbox::start_prompt_inbox_polling()
       │
2. Spawns background tokio task polling every `config.prompt_inbox_interval_ms`
       │
3. Each tick: poll_inbox_once(inbox_dir, state)
   ├─ scan directory for .txt files
   ├─ sort by mtime, pick oldest non-empty file
   ├─ check busy guard for "inbox" chat key
   ├─ state.begin_processing("inbox", content)
   ├─ sessions.get_or_create("inbox" context)
   ├─ session.prompt(content)  ← non-streaming, fire-and-forget
   ├─ state.end_processing("inbox")
   └─ delete processed .txt file
```

### Voice Flow

```
1. voice_handler receives voice/audio message
2. Download .ogg from Telegram API
3. Call voice::transcribe() → best available backend
4. Edit status message with transcript preview
5. Feed transcript to process_prompt() (same as text flow)
6. Cleanup temp file
```

### Photo/Document Flow

```
1. photo_handler receives photo/document message
2. Download image from Telegram API
3. state.begin_processing(key, caption)
4. sessions.get_or_create(ctx)
5. session.prompt_with_images(caption, &[img_path])
   └─ CliSession: spawns `pi --mode json --print @<path> <caption>`
6. Edit status message with response
7. Cleanup temp file
```

## Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Session abstraction | `PiSession` trait (async_trait) | Allows future direct-protocol impl beyond CLI subprocess |
| Streaming events | `PiEvent` enum + mpsc channel | Real-time progress to Telegram; thinking/tool/text deltas |
| JSON event parsing | `pi --mode json --print` → line-by-line serde | Structured streaming from CLI; maps directly to `PiEvent` |
| Debounced edits | 1.5s interval, skip if text unchanged | Avoid Telegram rate limits; reduce API calls during streaming |
| Per-chat session | `HashMap<SessionContext, Arc<dyn PiSession>>` | Each Telegram chat/thread gets isolated Pi state |
| Bootstrap session | First-created session consumes `PI_SESSION_PATH` | Enables resuming an existing Pi session |
| Chat busy guard | `BotChatState` with `Arc<Mutex<>>` | Prevents concurrent prompts per chat (Pi CLI is single-threaded) |
| Process abort | SIGTERM via `libc::kill` (Unix) | Graceful termination of running `pi` subprocess |
| Message splitting | `transport::split_text` at 4096 chars, newline-aware | Telegram API limit compliance |
| Config resolution | `telepi.toml` → env vars → .env | TOML as primary config; env vars override; `.env` as fallback |
| Voice backends | Priority: Parakeet > Sherpa-ONNX > OpenAI Whisper | Local-first, cloud fallback |
| Prompt inbox | Filesystem polling with configurable interval | External tools inject prompts by writing .txt files |
| Containerization | Multi-stage Docker build (rust:1.85-slim → debian:bookworm-slim) | Small runtime image, non-root user, volume mounts for config/sessions |
| 409 Conflict retry | Loop with 5 retries, 3s delay | Handles stale webhook or concurrent polling instances |
| Process cleanup | `kill_existing_processes()` on start | Kills previous telepi + orphan `pi --mode json` children before polling |
| Conversation tree | `pi/tree.rs` parses session JSONL into `TreeNode` graph | Enables /tree command to visualize Pi session history |

## PiSession Trait Interface

```rust
trait PiSession: Send + Sync {
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

## CliSession: JSON Event Pipeline

```
pi --mode json --print <text>
    │
    ▼ stdout (one JSON object per line)
    │
    ├─ JsonEvent::Session         → (ignored)
    ├─ JsonEvent::AgentStart      → (ignored)
    ├─ JsonEvent::TurnStart       → (ignored)
    ├─ JsonEvent::MessageStart    → (ignored)
    ├─ JsonEvent::MessageUpdate   → AssistantMessageEvent dispatch:
    │     ├─ ThinkingDelta        → PiEvent::ThinkingDelta
    │     ├─ TextDelta            → accumulate + PiEvent::TextDelta
    │     ├─ ToolStart            → PiEvent::ToolStart
    │     ├─ ToolOutput           → PiEvent::ToolOutput
    │     ├─ ToolEnd              → PiEvent::ToolEnd
    │     └─ (others)             → (ignored)
    ├─ JsonEvent::MessageEnd      → extract usage (tokens, cost)
    ├─ JsonEvent::TurnEnd         → extract final usage → PiEvent::Usage + PiEvent::TurnEnd
    ├─ JsonEvent::AgentEnd        → (ignored)
    └─ JsonEvent::Unknown         → (ignored)
```

## Unimplemented / Partial

| Feature | Status | Notes |
|---|---|---|
| Voice: Parakeet | Stub | `transcribe_parakeet()` returns error |
| Voice: Sherpa-ONNX | Stub | `transcribe_sherpa()` returns error |
| Voice: OpenAI Whisper | Working | Uses `OPENAI_API_KEY`, multipart upload |
| Photo handler | Working | Downloads image, calls `prompt_with_images()` |
| Abort | Working (Unix) | Sends SIGTERM to child process PID |
| /tree | Working | Parses session JSONL, renders tree via `pi/tree.rs` |
| /branch | Stub | "coming soon" — would navigate tree entries |
| /label | Stub | "coming soon" — would label tree entries |
| Prompt inbox | Working | Polls directory for .txt files, processes as prompts |
| Stats | Stub | Returns zeros (TODO: parse session JSONL) |
| set_model | Stub | No-op (TODO: persist selection) |
| setup command | Stub | Prints manual instructions |
| Service running check | Stub | `running: false` hardcoded in `get_status()` |
