# Architecture

## System Overview

Telegram bot that bridges user messages to the **Pi coding agent CLI** (`pi`).
One Rust binary, three runtime modes: `start` (bot), `setup` (interactive config), `status` (diagnostics).

```
┌─────────────┐      Telegram API      ┌──────────────────────────────────┐
│  Telegram    │ ◄──── polling ──────►  │  telepi binary                   │
│  User(s)     │                        │                                  │
└─────────────┘                        │  ┌──────────┐  ┌──────────────┐ │
                                        │  │  bot/    │  │  pi/         │ │
                                        │  │ handler  │─►│ registry     │ │
                                        │  │ commands │  │ session      │ │
                                        │  │ state    │  │ cli_session  │ │
                                        │  │ transport│  └──────┬───────┘ │
                                        │  │ keyboard │         │         │
                                        │  └──────────┘         ▼         │
                                        │               ┌──────────────┐  │
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
│    commands — /start, /new, /sessions, etc.      │
│    state    — per-chat busy/idle tracking        │
│    transport— send/edit/split Telegram messages   │
│    keyboard — paginated inline keyboards         │
├─────────────────────────────────────────────────┤
│  Session Layer          pi/                      │  ← PiSession trait, registry
│    registry — HashMap<SessionContext, PiSession>  │
│    session  — trait + data types + PiEvent enum  │
│    cli_session — pi CLI subprocess (json mode)   │
├─────────────────────────────────────────────────┤
│  Support Layer          config, paths, error,    │  ← shared utilities
│                         format, install, voice   │
└─────────────────────────────────────────────────┘
```

## Module Dependency Graph

```
main.rs
  └─► cli.rs
  └─► config.rs ──► paths.rs, error.rs
  └─► bot/mod.rs ──► config, pi/registry, bot/*

bot/mod.rs ──► bot/handler, bot/commands, bot/state, config, pi/registry
bot/handler.rs ──► bot/state, bot/transport, config, format, pi/registry, pi/session (PiEvent)
bot/commands/mod.rs ──► bot/handler (HandlerState)
bot/commands/sessions.rs ──► bot/handler (HandlerState), bot/state
bot/commands/basic.rs ──► bot/handler, config  [dead code — unused by dispatch()]
bot/commands/context.rs ──► bot/handler (HandlerState)
bot/commands/model.rs ──► bot/handler (HandlerState)
bot/commands/tree.rs ──► bot/handler (HandlerState)
bot/state.rs ──► pi/session (SessionContext)
bot/transport.rs ──► teloxide only
bot/keyboard.rs ──► teloxide only

pi/registry.rs ──► config, error, pi/session, pi/cli_session
pi/cli_session.rs ──► config, error, pi/session
pi/session.rs ──► (pure trait + types, no internal deps)

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
| Config resolution | `TELEPI_CONFIG` → .env (cwd) → ~/.config/telepi/.env | Docker-friendly, explicit override path |
| Voice backends | Priority: Parakeet > Sherpa-ONNX > OpenAI Whisper | Local-first, cloud fallback |

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
| Stats | Stub | Returns zeros (TODO: parse session JSONL) |
| set_model | Stub | No-op (TODO: persist selection) |
| /tree, /branch, /label | Stubs | "not yet implemented" messages |
| commands::basic.rs | Dead code | `cmd_start`/`cmd_help` defined but `dispatch()` uses inline `send_welcome()` |
| Prompt inbox | Configured but unused | `prompt_inbox_dir` / `prompt_inbox_interval_ms` in config, no watcher |
| setup command | Stub | Prints manual instructions |
| Service running check | Stub | `running: false` hardcoded in `get_status()` |
