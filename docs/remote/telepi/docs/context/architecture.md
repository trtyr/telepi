# TelePi Architecture

## System Overview

TelePi is a **Telegram bridge** for the Pi coding agent (`@mariozechner/pi-coding-agent`). It wraps Pi's `AgentSessionRuntime` in a grammY-based Telegram bot, adding per-chat session isolation, voice transcription, extension UI bridging, and installed-mode service management.

```
┌─────────────────────────────────────────────────────┐
│                   Telegram API                       │
└──────────────────────┬──────────────────────────────┘
                       │ webhooks / long-polling
┌──────────────────────▼──────────────────────────────┐
│               Bot Layer (grammY)                     │
│  bot.ts → dispatcher → command handlers              │
│         → message:text → prompt handler              │
│         → message:voice → voice → prompt handler     │
│         → message:photo → image → prompt handler     │
│         → callback_query → keyboard / dialogs        │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│           Session Layer (PiSessionRegistry)          │
│  per-chat PiSessionService → AgentSessionRuntime     │
│  model-scope │ tree │ provider-response-notices      │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│         Pi SDK (@mariozechner/pi-coding-agent)       │
│  AgentSession │ SessionManager │ ModelRegistry       │
│  AuthStorage  │ SettingsManager │ ExtensionRunner    │
└──────────────────────┬──────────────────────────────┘
                       │
              ┌────────▼────────┐
              │  LLM Providers  │
              │  (API calls)    │
              └─────────────────┘
```

## Layering Model

| Layer | Scope | Key Files |
|-------|-------|-----------|
| **Entrypoints** | Boot & CLI dispatch | `src/index.ts`, `src/cli.ts`, `src/entrypoint.ts` |
| **Config** | Env loading, workspace resolution | `src/config.ts`, `src/paths.ts` |
| **Bot** | grammY bot factory, dispatcher, command wiring | `src/bot.ts` |
| **Bot Submodules** | Prompt flow, transport, state, rendering | `src/bot/*.ts` (14 files) |
| **Session** | Per-chat Pi agent lifecycle | `src/pi-session.ts`, `src/pi-session-paths.ts` |
| **Model** | Model scoping, selection, thinking levels | `src/model-scope.ts` |
| **Voice** | Audio transcription (parakeet / sherpa / openai) | `src/voice.ts` |
| **Tree** | Session tree rendering for Telegram | `src/tree.ts` |
| **UI Bridge** | Pi ExtensionUIContext → Telegram shim | `src/telegram-ui-context.ts`, `src/bot/extension-dialogs.ts` |
| **Install** | Setup, service management, extension install | `src/install.ts`, `src/install/*.ts` |
| **Shared** | Error formatting, HTML escaping, provider notices | `src/errors.ts`, `src/format.ts`, `src/provider-response-notices.ts` |

## Module Dependency Graph

```
cli.ts ──────────────┐
                     │
index.ts ──► bot.ts ──┼──► config.ts ──► paths.ts
              │       │
              │       ├──► pi-session.ts ──► model-scope.ts
              │       │         │
              │       │         ├──► pi-session-paths.ts
              │       │         └──► tree.ts
              │       │
              │       ├──► voice.ts ──► install/platform.ts
              │       │
              │       └──► bot/*.ts
              │            ├── prompt-handler.ts ──► message-rendering.ts
              │            ├── telegram-transport.ts
              │            ├── chat-state.ts
              │            ├── chat-task-runner.ts
              │            ├── slash-command.ts
              │            ├── command-picker.ts
              │            ├── keyboard.ts
              │            ├── extension-dialogs.ts
              │            ├── prompt-inbox.ts
              │            ├── tree-callbacks.ts
              │            ├── callback-query-logging.ts
              │            └── commands/{basic,sessions,context,model,tree}.ts
              │
              └──► telegram-ui-context.ts
                   provider-response-notices.ts
                   errors.ts
                   format.ts
                   callback-data.ts

install.ts ──► install/{config,extension,launchd,systemd,platform,shared,service-manager,clipboard}.ts
```

## Data Flow

### Text Message → LLM → Reply

```
1. Telegram message:text
        │
2. bot.ts dispatcher: extract target (chatId + messageThreadId)
        │
3. Check: is local TelePi command? → route to bot.command handler
   Check: is Pi slash command? → rewrite + pass through
   Check: extension dialog pending? → consume input
        │
4. handleUserPrompt(ctx, target, userText)
        │
5. ChatTaskRunner.tryStartPrompt()
   - Check busy → reply "busy" if occupied
   - beginProcessing(target, promptText)
        │
6. PiSessionRegistry.getOrCreate(target)
   - key = "chatId::messageThreadId"
   - Lazy-create PiSessionService → createPiSession() → AgentSessionRuntime
   - Bootstrap session path consumed by first chat only
        │
7. PiSessionService.prompt(text, images?)
   → AgentSession.prompt(text, images)
        │
8. PiSessionCallbacks stream back:
   onTextDelta  → debounced edit to Telegram message (EDIT_DEBOUNCE_MS=1500)
   onToolStart  → send "🔧 tool_name" message
   onToolUpdate → edit with partial output (configurable verbosity)
   onToolEnd    → edit with final status
   onAgentEnd   → endProcessing()
        │
9. Telegram reply sent/edited via telegram-transport.ts
```

### Voice Message Flow

```
message:voice → downloadTelegramFile() → transcribeAudio(file)
  → backend: parakeet | sherpa-onnx | openai (fallback chain)
  → transcript text → handleUserPrompt() → same as text flow
```

### Image Message Flow

```
message:photo|document → selectPhotoFileId() → downloadTelegramFile()
  → resolve MIME type → build ImageContent[]
  → handleUserPrompt(ctx, target, caption, undefined, images)
```

### Prompt Inbox (External Injection)

```
Filesystem polling (TELEPI_PROMPT_INBOX_DIR)
  → startPromptInboxPolling() reads .txt files from inbox dir
  → If not busy: claims file, calls handlePrompt(), acks (deletes) file
  → Enables programmatic prompt injection without Telegram
```

## Key Design Decisions

### Per-Chat Session Isolation

Sessions are keyed by `chatId::messageThreadId` (see `getPiSessionContextKey()` in `src/pi-session.ts:1229`). Each Telegram chat (and topic thread) gets its own `PiSessionService` wrapping a separate `AgentSessionRuntime`. This prevents cross-chat state contamination.

### Bootstrap Session Path

`PiSessionRegistry` holds a single `bootstrapSessionPath` consumed by the first `getOrCreate()` call (`src/pi-session.ts:1334`). Subsequent chats always create fresh sessions. This allows resuming an existing Pi CLI session via `PI_SESSION_PATH` env but only for the first connected chat.

### Bash Tool Timeout Patching

Pi SDK's bash tool has no default timeout. TelePi monkey-patches the tool after session creation (`src/pi-session.ts:361`, `patchBashTimeout()`) to enforce 120s default. Also blocks `launchctl` commands targeting `com.telepi` to prevent self-restart (`src/pi-session.ts:329`).

### Busy-Guard via ChatTaskRunner

`ChatTaskRunner` (`src/bot/chat-task-runner.ts`) enforces one-active-prompt-per-chat. `BotChatState` (`src/bot/chat-state.ts`) tracks processing/switching/transcribing states per context key. Busy state is checked before accepting new messages, voice, or images.

### Extension UI Bridging

Pi's `ExtensionUIContext` expects an interactive terminal. TelePi provides a Telegram shim (`src/telegram-ui-context.ts`) that maps `select/confirm/input` to inline-keyboard dialogs (`src/bot/extension-dialogs.ts`) with timeout and abort support. Unsupported methods (editor, custom, theme) throw or no-op.

### Platform Abstraction for Installed Mode

The `install/` subdirectory abstracts macOS (launchd via `launchd.ts`) and Linux (systemd via `systemd.ts`) behind a `ServiceManager` interface (`install/service-manager.ts`). `install/platform.ts` auto-detects the platform and assembles a `TelePiInstallContext` with all paths.

### Config Resolution Chain

Config path resolution (`src/config.ts:65`, `getConfigEnvPathInfo()`):
1. `TELEPI_CONFIG` env (explicit)
2. `.env` in cwd
3. `~/.config/telepi/config.env` (default)

Workspace resolution (`src/config.ts:112`):
1. Docker: `/workspace`
2. `TELEPI_WORKSPACE` env
3. `process.cwd()`

### Streaming Edit Debounce

Text deltas from the LLM are debounced at 1500ms before editing the Telegram message (`EDIT_DEBOUNCE_MS` in `src/bot.ts:71`). This avoids Telegram's rate limits while keeping the response feeling live. A "typing" action is sent every 4500ms (`TYPING_INTERVAL_MS`).

### Polling Restart with 409 Handling

Long-polling restarts on HTTP 409 Conflict (another bot instance) up to 5 attempts with 3s delay (`src/index.ts:6-7`). This handles stale polling sessions gracefully.

## Tradeoffs

| Decision | Tradeoff |
|----------|----------|
| Per-chat session isolation | More memory per chat; no shared context across chats |
| Monkey-patch bash timeout | Fragile if Pi SDK changes tool internals; but no API for default timeout |
| Bootstrap session consumed once | First chat gets the session; deterministic but surprising if multiple chats connect simultaneously |
| grammY long-polling | Simpler than webhooks; slight latency; 409 retry handles conflicts |
| Inline-keyboard dialogs for extensions | Limited to select/confirm/input; no rich terminal UI; timeouts required |
| Prompt inbox filesystem polling | Simple external injection; no real-time; requires filesystem access |

## Entry Points

| File | Purpose | How It Starts |
|------|---------|---------------|
| `src/index.ts` | Bot process | `node dist/index.ts` or `npm start` |
| `src/cli.ts` | CLI (`telepi start/setup/status/version/help`) | `telepi` command |
| `src/entrypoint.ts` | ESM entrypoint detection | Used by index.ts and cli.ts |

## Start Here

`src/index.ts` — the boot sequence: `loadConfig()` → `PiSessionRegistry.create()` → `createBot()` → `registerCommands()` → `bot.start()`. From there, `src/bot.ts` for all wiring, `src/pi-session.ts` for session lifecycle, and `src/bot/prompt-handler.ts` for the core prompt→response flow.
