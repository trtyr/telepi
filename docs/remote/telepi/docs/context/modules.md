# TelePi Module Reference

Source modules grouped by functional area. Tables list public exports.

---

## Bot / Telegram Layer

### `src/bot.ts` — Composition Root

Assembles `Bot<Context>`, wires all handlers, state maps, slash commands.
Exports: `createBot(config, sessionRegistry) → Bot<Context>`, `registerCommands(bot)`.
Pattern: single factory closing over ~20 `Map`s for callback-keyed UI state.
Deps: `config`, `errors`, `format`, `pi-session`, `tree`, `voice`, all `bot/*` modules.

### `src/bot/chat-state.ts` — Busy Tracking

`BotChatState` interface, `createBotChatState()`. Tracks
processing/switching/transcribing per context. Deps: `pi-session` (types).

### `src/bot/chat-task-runner.ts` — Serialised Prompt Tasks

`ChatTaskRunner` (`tryStartPrompt`), `createChatTaskRunner(deps)`.
Rejects overlapping prompts as "busy". Deps: `pi-session` (types).

### `src/bot/telegram-transport.ts` — API Primitives

`TextOptions`, `getTelegramTarget`, `safeReply`, `safeEditMessage`,
`sendTextMessage`, `sendChatAction`, `downloadTelegramFile`. Handles 4096-char
limits and parse-mode fallback. Deps: `grammy`, `message-rendering`, `pi-session` (types).

### `src/bot/prompt-handler.ts` — Prompt Dispatch

`HandleUserPrompt` type, `createPromptHandler(deps)`. Streams
tool/text deltas to Telegram; handles extension UI dialogs.
Deps: `config`, `errors`, `format`, `pi-session`, `telegram-ui-context`, `bot/*`.

### `src/bot/prompt-inbox.ts` — File-Based Prompt Ingestion

`startPromptInboxPolling(options)` → stop fn. Polls directory for `.txt` prompts
(Pi CLI hand-off). Deps: `pi-session` (types).

### `src/bot/message-rendering.ts` — Telegram HTML Rendering

| Export | Kind |
|---|---|
| `TelegramParseMode`, `RenderedText`, `RenderedChunk` | types |
| `TELEGRAM_MESSAGE_LIMIT` (4000), `TOOL_OUTPUT_PREVIEW_LIMIT` (500) | constants |
| `renderHelp{Plain,HTML}`, `renderSessionInfo{Plain,HTML}` | functions |
| `renderToolStartMessage/End`, `renderFailedText`, `renderPrefixedError` | functions |
| `buildStreamingPreview`, `splitTelegramText`, `isMessageNotModifiedError` | functions |

Deps: `errors`, `format`, `pi-session` (types).

### `src/bot/keyboard.ts` — Inline Keyboard Pagination

`KeyboardItem`, `KEYBOARD_PAGE_SIZE` (6), `paginateKeyboard`, `appendKeyboardItems`.
Deps: `grammy`, `callback-data`.

### `src/bot/slash-command.ts` — Command Metadata

`TELEPI_BOT_COMMANDS`, `normalizeSlashCommand`, `rewriteSlashCommandForTelegram`,
`buildChatScopedCommands`, `getTelepiNativeCommandMenu`, `CommandPickerEntry`,
`buildCommandPickerEntries`, `filterCommandPickerEntries`.
Deps: `@mariozechner/pi-coding-agent`, `message-rendering`.

### `src/bot/extension-dialogs.ts` — Interactive Extension Dialogs

`PendingExtensionDialog`, `ExtensionDialogManager`, `createExtensionDialogManager(deps)`.
Manages select/confirm/input dialogs with timeout and abort.
Deps: `grammy`, `pi-session` (types).

### `src/bot/callback-query-logging.ts` — Callback Error Logging

`COMMAND_MENU_CALLBACK_PREFIX`, `isStaleCallbackQueryError`, `logCallbackQueryError`.

### `src/bot/tree-callbacks.ts` — Tree Navigation Callbacks

`PendingTreeView`, `registerTreeCallbacks(deps)`. Deps: `format`, `pi-session`, `tree`.

### `src/bot/command-picker.ts` — /commands Picker

`PendingCommandPicker`, `createCommandPickerHandlers(deps)`. Deps: `format`, `pi-session`, `bot/*`.

### `src/bot/commands/*.ts` — Command Handler Groups

Each exports a single `create*CommandHandlers(deps)` factory returning a handler map.

- `basic.ts` → `/start`, `/help`, `/abort`, `/retry`, `/session`
- `context.ts` → `/context` (context window usage)
- `model.ts` → `/model` (model picker and switching)
- `sessions.ts` → `/sessions`, `/new`, `/handback`
- `tree.ts` → `/tree`, `/branch`, `/label`

All depend on `format`, `pi-session` (types), and `bot/message-rendering`.

---

## Pi Session Layer

### `src/pi-session.ts` — Session Lifecycle & Registry

Largest module (~1528 lines). Wraps `@mariozechner/pi-coding-agent` SDK.
Patches bash tool (timeout 120s, self-management guard).

| Export | Kind |
|---|---|
| `PiSessionContext` | interface (`chatId`, `messageThreadId?`) |
| `PiSessionCallbacks`, `PiSessionDiagnostic`, `PiSessionInfo` | interfaces |
| `PiSessionSwitchResult`, `PiSessionModelOption`, `ResolvedSessionReference` | interfaces |
| `PiSessionNewSessionOptions`, `PiSessionSwitchOptions`, `PiSessionForkOptions` | types |
| `subscribeToSession`, `getPiSessionContextKey` | functions |
| `PiSessionService` | class (create/getSession/isStreaming/newSession/switchSession/prompt/abort/setModel/getModels/getContextUsage/getSessionStats/resolveSessionReference/getTree/getLabels/setLabel/dispose) |
| `PiSessionRegistry` | class (create/has/get/getInfo/getOrCreate/remove/dispose) |

Deps: `@mariozechner/pi-coding-agent`, `@mariozechner/pi-agent-core`, `@mariozechner/pi-ai`, `config`, `model-scope`, `pi-session-paths`, `provider-response-notices`, `telegram-ui-context`, `tree`.

### `src/model-scope.ts` — Model Glob Resolution

`ScopedModelOption`, `resolveScopedModels`, `resolveInitialScopedModelSelection`.
Resolves `enabledModels` globs with thinking-level suffixes.
Deps: `@mariozechner/pi-coding-agent`, `@mariozechner/pi-agent-core`, `@mariozechner/pi-ai`, `minimatch`.

### `src/pi-session-paths.ts` — Session Path Resolution

`resolveSessionPathForRuntime`, `readSessionHeader`, `resolveWorkspacePathForRuntime`.
Remaps host paths to Docker container paths. Deps: `paths`.

### `src/telegram-ui-context.ts` — Extension UI Shim

`TelegramExtensionNoticeType`, `createTelegramUIContext(options) → ExtensionUIContext`.
Plain-text shim for Pi SDK's UI context. Deps: `@mariozechner/pi-coding-agent` (types).

### `src/provider-response-notices.ts` — Provider Error Notices

`ProviderResponseNoticeEvent`, `ProviderResponseNotice`, `getProviderResponseNotice`,
`createProviderResponseNoticeExtension`. Deps: `@mariozechner/pi-coding-agent` (types), `telegram-ui-context`.

### `src/tree.ts` — Session Tree Rendering

`SessionTreeNodeLike`, `TreeButton`, `TreeRenderResult`, `TreeFilterMode`,
`truncateText`, `renderTree`, `describeEntry`, `renderBranchConfirmation`, `renderLabels`.
Deps: `@mariozechner/pi-coding-agent` (types), `callback-data`, `format`.

---

## Install / Service Layer

### `src/install.ts` — Setup & Status Facade

`setupTelePi → TelePiSetupResult`, `getTelePiStatus → TelePiStatus`.
Re-exports all shared types and submodule helpers.
Deps: `install/config`, `install/extension`, `install/launchd`, `install/platform`, `install/shared`.

### `src/install/shared.ts` — Shared Types & Constants (leaf)

Constants: `TELEPI_LAUNCHD_LABEL`, `TELEPI_SERVICE_NAME`, `TELEPI_EXTENSION_FILENAME`.
Types: `PlatformIdentifier`, `TelePiInstallContext`, `ServiceStatus`, `ExtensionStatus`,
`TelePiStatus`, `TelePiSetupOptions`, `TelePiSetupResult`, `TelePiConfigSetupValues`.

### `src/install/platform.ts` — Platform Detection & Context

`detectPlatform`, `resolveTelePiInstallContext`, `getServiceManager`, `getPlatformInstallHint`.
Deps: `paths`, `install/launchd`, `install/systemd`, `install/shared`.

### `src/install/service-manager.ts` — Service Manager Interface (leaf)

`ServiceManager` interface (`buildUnitFile`/`writeUnitFile`/`reconcile`/`getStatus`).
Implemented by `launchd.ts` and `systemd.ts`.

### `src/install/launchd.ts` — macOS LaunchAgent

`buildLaunchAgentPlist`, `getInstalledConfigStatus`, `createLaunchdManager`.
Deps: `paths`, `install/service-manager`, `install/shared`.

### `src/install/systemd.ts` — Linux systemd

`buildSystemdUnit`, `writeSystemdUnit`, `createSystemdManager`.
Deps: `install/service-manager`, `install/shared`.

### `src/install/config.ts` — Config File Management

`ensureTelePiConfig → TelePiConfigSetupResult`, `getServiceConfigSource`.
Deps: `config`, `paths`, `install/shared`.

### `src/install/extension.ts` — Extension Installation

`installExtension → "symlink" | "copy"`, `getExtensionStatus → ExtensionStatus`.

### `src/install/clipboard.ts` — System Clipboard (leaf)

`copyToClipboard(text) → boolean`. Uses `pbcopy`/`wl-copy`/`xclip`/`xsel`.

---

## Voice Layer

### `src/voice.ts` — Audio Transcription

`TranscriptionResult`, `TranscriptionBackend`, `transcribeAudio(filePath)`,
`getAvailableBackends`, `getVoiceBackendStatus`. Fallback chain:
parakeet-coreml → sherpa-onnx → OpenAI Whisper. Uses ffmpeg for decoding,
mutex guards for native engine serialisation. Deps: `install/platform`.

---

## Utilities

### `src/config.ts` — Configuration Loader

`TelePiConfig`, `ToolVerbosity`, `loadConfig()`, `getConfigEnvPathInfo`,
`parseAllowedUserIds`. Deps: `paths`.

### `src/paths.ts` — Path Utilities (leaf)

`DOCKER_WORKSPACE_PATH`, `getHomeDirectory`, `expandHomePath`, `resolvePathFromCwd`,
`getDefaultTelePiConfigPath`, `getDefaultSystemdUserDir`, `getDefaultLogDir`.

### `src/format.ts` — Markdown → Telegram HTML (leaf)

`escapeHTML(text)`, `formatTelegramHTML(markdown)`.

### `src/errors.ts` — Error Formatting (leaf)

`formatError(error)`, `toFriendlyError(error)`.

### `src/entrypoint.ts` — ESM Entry Guard (leaf)

`isEntrypoint(moduleUrl, argvPath?) → boolean`.

### `src/callback-data.ts` — Callback Constants (leaf)

`NOOP_PAGE_CALLBACK_DATA` = `"noop_page"`.

---

## Orchestration Entrypoints

### `src/index.ts` — Bot Startup

`startBot()`: creates `PiSessionRegistry`, composes bot, starts polling with
409 auto-restart (max 5 attempts), handles SIGINT/SIGTERM.

### `src/cli.ts` — CLI

`main(argv?)`: dispatches `start | setup | status | version | help`.
