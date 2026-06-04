# TelePi Module Reference

TypeScript source modules grouped by functional area. All modules use factory functions with closure-based DI — no classes.

---

## Bot / Telegram Layer

### `src/bot.ts` — Main Bot Wiring (~1316 lines)

Composition root. Creates Grammy `Bot`, registers all command handlers, wires callback query routing, sets up the `PiSessionRegistry` and `BotChatState`, starts polling with auto-retry.

Key exports: `createBot(deps)` — factory function.

Deps: `config`, `pi-session`, all `bot/*` submodules, `voice`, `format`, `model-scope`.

### `src/bot/chat-state.ts` — Per-Chat Busy Tracking

| Export | Kind |
|--------|------|
| `BotChatState` | interface |
| `createBotChatState()` | factory → `BotChatState` |

Methods: `isLocallyBusy(target)`, `beginProcessing(target, prompt)`, `endProcessing(target)`, `beginSwitching(target)`, `endSwitching(target)`, `beginTranscribing(target)`, `endTranscribing(target)`, `getLastPrompt(target)`, `clearPromptMemory(target)`.

Uses closure-captured `Set<string>` for state. No classes.

### `src/bot/chat-task-runner.ts` — Async Task Execution

| Export | Kind |
|--------|------|
| `ChatTaskRunner` | interface |
| `createChatTaskRunner(deps)` | factory → `ChatTaskRunner` |

Wraps prompt execution with busy-guard logic. Integrates with `BotChatState` to prevent concurrent prompts per chat.

### `src/bot/prompt-handler.ts` — Prompt Flow Orchestration

Handles the full prompt lifecycle: validate → mark busy → send typing → send "🤔 Thinking..." → call `PiSessionService.prompt()` → format response → edit message → mark idle.

Deps: `chat-state`, `telegram-transport`, `message-rendering`, `pi-session`.

### `src/bot/telegram-transport.ts` — Safe Telegram API Wrappers

| Export | Kind |
|--------|------|
| `sendReply(ctx, text, options)` | safe reply with error handling |
| `editMessage(ctx, messageId, text)` | edit with retry |
| `sendTyping(ctx)` | typing indicator |
| `downloadFile(ctx, fileId)` | download voice/photo files |

Handles Telegram rate limits via `@grammyjs/auto-retry`.

### `src/bot/message-rendering.ts` — HTML Formatting & Chunking

| Export | Kind |
|--------|------|
| `renderPromptResponse(response)` | format Pi response as Telegram HTML |
| `splitMessage(text, limit)` | split at 4096 chars, newline-aware |
| `ContextUsageInfo` | interface |
| `SessionStatsInfo` | interface |

### `src/bot/keyboard.ts` — Inline Keyboard Pagination

| Export | Kind |
|--------|------|
| `KeyboardItem` | type `{ label, callbackData }` |
| `paginateKeyboard(items, page, pageSize)` | paginated inline keyboard |

### `src/bot/slash-command.ts` — Command Normalization

| Export | Kind |
|--------|------|
| `NormalizedSlashCommand` | type `{ name, text }` |
| `CommandPickerFilter` | type `"all" \| "telepi" \| "pi"` |
| `normalizeSlashCommand(text)` | parse raw text into command structure |

### `src/bot/commands/` — Grouped Command Handlers

| File | Commands | Notes |
|------|----------|-------|
| `basic.ts` | `/start`, `/help`, `/new` | Welcome, session creation |
| `sessions.ts` | `/sessions`, `/session` | List/switch sessions |
| `model.ts` | `/model` | Model picker with inline keyboard |
| `tree.ts` | `/tree`, `/branch`, `/label` | Session tree navigation |
| `context.ts` | `/context` | Context window usage |
| `tree-callbacks.ts` | tree callback handlers | Navigation, pagination, filtering |
| `command-picker.ts` | `/commands` picker | Paginated TelePi + Pi command browser |

### `src/bot/extension-dialogs.ts` — Telegram-Backed Extension UI

| Export | Kind |
|--------|------|
| `handleSelectDialog(ctx, options)` | Telegram inline keyboard select |
| `handleConfirmDialog(ctx, message)` | yes/no confirm |
| `handleInputDialog(ctx, placeholder)` | wait for text input |

Allows Pi extension commands to open Telegram-native dialogs mid-execution.

### `src/bot/callback-query-logging.ts` — Stale Callback Suppression

Suppresses duplicate "query too old" error logs. `STALE_CALLBACK_LOG_WINDOW_MS = 30000`.

---

## Pi Session Layer

### `src/pi-session.ts` — Pi SDK Session Wrapper (~1528 lines)

Core session management. Wraps the Pi SDK's `SessionManager` and `AgentSession`.

| Export | Kind |
|--------|------|
| `PiSessionContext` | interface `{ chatId, messageThreadId? }` |
| `PiSessionInfo` | interface `{ sessionId, sessionFile?, workspace, model?, ... }` |
| `PiSessionService` | class — per-session operations |
| `PiSessionRegistry` | class — session store with `getOrCreate()` |
| `PiSessionCallbacks` | interface — streaming callbacks |
| `getPiSessionContextKey(ctx)` | string key for chat/topic |
| `consumeBootstrapSessionPath()` | one-shot bootstrap path consumption |

Key behaviors:
- **Bootstrap path**: `PI_SESSION_PATH` consumed by first `getOrCreate()` call, then ignored
- **Session storage**: `~/.pi/agent/sessions/<encoded-workspace>/`
- **Workspace switching**: re-scopes coding tools via `createCodingTools(workspace)`
- **Self-management block**: blocks `launchctl` commands targeting `com.telepi` from inside sessions

---

## Install / Service Layer

### `src/install.ts` — Public Facade

Re-exports setup and status functions. Thin orchestration layer excluded from coverage.

### `src/install/platform.ts` — Platform Detection

Platform detection (`darwin` → macOS, `linux` → Linux), install context resolution.

### `src/install/config.ts` — Config File Management

Creates/updates `~/.config/telepi/config.env`. Preserves existing optional values on re-setup.

### `src/install/launchd.ts` — macOS LaunchAgent

Generates plist from template, installs to `~/Library/LaunchAgents/com.telepi.plist`, manages via `launchctl`.

### `src/install/systemd.ts` — Linux systemd User Service

Generates unit file from template, installs to `~/.config/systemd/user/telepi.service`, manages via `systemctl --user`.

### `src/install/service-manager.ts` — Shared Service Interface

Common interface for launchd/systemd operations (install, uninstall, restart, status).

### `src/install/extension.ts` — Pi Extension Install

Symlinks `extensions/telepi-handoff.ts` to `~/.pi/agent/extensions/`.

### `src/install/clipboard.ts` — Cross-Platform Clipboard

`copyToClipboard(text)` — tries `pbcopy` (macOS), `wl-copy`, `xclip`, `xsel` (Linux).

### `src/install/shared.ts` — Shared Types/Constants

Common types and constants used across install modules.

---

## Voice Layer

### `src/voice.ts` — Audio Transcription

| Export | Kind |
|--------|------|
| `TranscriptionResult` | interface `{ text, backend, durationMs }` |
| `TranscriptionBackend` | type `"parakeet" \| "sherpa-onnx" \| "openai"` |
| `availableBackends()` | checks which backends are available |
| `transcribeAudio(filePath)` | tries best available backend |

Fallback chain: Parakeet CoreML → Sherpa-ONNX → OpenAI Whisper. All three implemented. Downloads voice to temp dir, transcribes, deletes immediately.

---

## Utilities

### `src/config.ts` — Configuration Loader

Hand-rolled `.env` parser (no dotenv dependency). Supports `export KEY=VALUE` syntax.

| Export | Kind |
|--------|------|
| `TelePiConfig` | interface |
| `ToolVerbosity` | type `"all" \| "summary" \| "errors-only" \| "none"` |
| `loadConfig()` | reads config from env → `.env` (cwd) → `~/.config/telepi/config.env` |

### `src/errors.ts` — Error Helpers

| Export | Kind |
|--------|------|
| `formatError(error)` | extract message string for internal logging |
| `toFriendlyError(error)` | user-facing error messages, strips internal prefixes |

No custom error classes. Plain `Error` + string matching.

### `src/format.ts` — Markdown → Telegram HTML

Converts markdown to Telegram HTML. Pipeline: escape HTML → extract code blocks → bold → italic → links → blockquotes → restore placeholders. Uses Unicode private-use-area characters for placeholder protection.

### `src/tree.ts` — Session Tree Rendering

| Export | Kind |
|--------|------|
| `SessionTreeNodeLike` | interface |
| `TreeFilterMode` | type `"default" \| "user-only" \| "all-with-buttons"` |
| `TreeRenderResult` | interface `{ text, buttons, totalEntries, ... }` |

### `src/model-scope.ts` — Model Filtering

| Export | Kind |
|--------|------|
| `ScopedModelOption` | interface `{ model, thinkingLevel? }` |
| `getScopedModels(session)` | filter/group available models |

### `src/telegram-ui-context.ts` — Extension UI Adapter

Adapts Pi extension UI calls to Telegram dialogs. Bridges `select`, `confirm`, `input` to inline keyboards and message waiting.

### `src/paths.ts` — Path Utilities

Cross-platform path resolution. `DOCKER_WORKSPACE_PATH`, `expandHome()`, `resolveFromCwd()`, default config/service/log directories.

### `src/entrypoint.ts` — ESM Entrypoint Guard

`isEntrypoint(moduleUrl)` — ESM equivalent of `if (require.main === module)`.

---

## Orchestration Entrypoints

### `src/index.ts` — Bot Startup

Graceful shutdown, polling restart with backoff (5 attempts, 3s delay on 409 Conflict). Excluded from coverage.

### `src/cli.ts` — CLI Commands

`start`, `setup`, `status`, `version`. Uses switch/case dispatch.

---

*Updated from TypeScript source on 2026-06-04.*
