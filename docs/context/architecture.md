# Architecture

## System Overview

Telegram bot that bridges user messages to the **Pi coding agent CLI** (`pi`).
One Rust binary, three runtime modes: `start` (bot), `setup` (interactive config), `status` (diagnostics).

```
┌─────────────┐      Telegram API      ┌──────────────────────────────┐
│  Telegram    │ ◄──── polling ──────►  │  telepi binary               │
│  User(s)     │                        │                              │
└─────────────┘                        │  ┌──────────┐  ┌──────────┐ │
                                        │  │  bot/    │  │  pi/     │ │
                                        │  │ handler  │─►│ registry │ │
                                        │  │ commands │  │ session  │ │
                                        │  │ state    │  └────┬─────┘ │
                                        │  │ transport│       │       │
                                        │  └──────────┘       ▼       │
                                        │              ┌──────────┐   │
                                        │              │ pi CLI   │   │
                                        │              │ (spawn)  │   │
                                        │              └──────────┘   │
                                        └──────────────────────────────┘
```

## Layering Model

```
┌─────────────────────────────────────────────────┐
│  CLI / Entry           main.rs, cli.rs           │  ← clap arg parsing, dispatch
├─────────────────────────────────────────────────┤
│  Bot Layer              bot/                     │  ← teloxide dispatcher, commands
│    handler  — message endpoints                  │
│    commands — /start, /new, /sessions, etc.      │
│    state    — per-chat busy/idle tracking        │
│    transport— send/edit/split Telegram messages   │
│    keyboard — paginated inline keyboards         │
├─────────────────────────────────────────────────┤
│  Session Layer          pi/                      │  ← PiSession trait, registry
│    registry — HashMap<SessionContext, PiSession>  │
│    session  — trait + data types                 │
│    cli_session — pi CLI subprocess impl          │
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

bot/handler.rs ──► bot/state, bot/transport, config, format, pi/registry
bot/commands/*.rs ──► bot/handler, bot/state, pi/registry, config
bot/state.rs ──► pi/session (SessionContext)
bot/transport.rs ──► teloxide only
bot/keyboard.rs ──► teloxide only

pi/registry.rs ──► config, pi/session, pi/cli_session
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

```
1. teloxide Dispatcher receives Message
       │
2. Filter chain selects handler branch:
   ┌──────────────────┬───────────────────┬──────────────┐
   │ command message   │ voice/audio msg   │ text msg     │
   │ → commands::dispatch → voice_handler  │ → text_handler│
   └──────────────────┴───────────────────┴──────┬───────┘
                                                  │
3. text_handler:                                   ▼
   ├─ auth check (config.is_allowed_user)
   ├─ busy check (state.is_busy)  ──► reject if Processing
   ├─ state.begin_processing(key, prompt)
   ├─ transport.send_typing()
   └─ process_prompt()
       │
4. process_prompt:                               ▼
   ├─ state::chat_key_to_context(key) → SessionContext
   ├─ sessions.get_or_create(ctx)
   │      └─ first call: CliSession::create(config, ctx)
   │           └─ creates session dir or uses bootstrap path
   ├─ bot.send_message("🤔 Thinking...")
   ├─ session.prompt(text)
   │      └─ CliSession: spawns `pi --prompt <text>` subprocess
   │           └─ stdout captured as PromptResponse.text
   ├─ format::escape_html(response)
   └─ transport::edit_text("🤔 Thinking..." → "<b>Pi:</b>\n...")

5. state.end_processing(key)
```

## Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Session abstraction | `PiSession` trait (async_trait) | Allows future direct-protocol impl beyond CLI subprocess |
| Per-chat session | `HashMap<SessionContext, Arc<dyn PiSession>>` | Each Telegram chat/thread gets isolated Pi state |
| Bootstrap session | First-created session consumes `PI_SESSION_PATH` | Enables resuming an existing Pi session |
| Chat busy guard | `BotChatState` with `Arc<Mutex<>>` | Prevents concurrent prompts per chat (Pi CLI is single-threaded) |
| Message splitting | `transport::split_text` at 4096 chars, newline-aware | Telegram API limit compliance |
| Config resolution | env var → .env (cwd) → ~/.config/telepi/.env | Docker-friendly, explicit override path |
| Voice backends | Priority: Parakeet > Sherpa-ONNX > OpenAI Whisper | Local-first, cloud fallback |

## Unimplemented (TODO)

- `voice_handler` in handler.rs — downloads voice but no transcription
- `photo_handler` — placeholder only
- `abort` in cli_session — no process kill yet
- `set_model` — not persisted
- `stats` — returns zeros
- `/tree`, `/branch`, `/label` commands — stubs
- `commands::basic.rs` — `cmd_start`/`cmd_help` defined but unused (dispatch uses `send_welcome` in mod.rs)
