# TelePi Module Reference

TypeScript source modules grouped by functional area. All modules use factory functions with closure-based DI — no classes.

---

## Bot / Telegram Layer

### `src/bot.ts` — Main Bot Wiring (~1316 lines)

Composition root. Creates Grammy `Bot`, registers all command handlers, wires callback query routing, sets up `PiSessionRegistry` and `BotChatState`, starts polling with auto-retry. Supports photo messages (image analysis) and prompt inbox polling.

Key exports: `createBot(config, sessionRegistry)`, `registerCommands(bot)`.

Deps: `config`, `pi-session`, all `bot/*` submodules, `voice`, `format`, `model-scope`, `prompt-inbox`.

### `src/bot/chat-state.ts` — Per-Chat Busy Tracking

| Export | Kind |
|--------|------|
| `BotChatState` | interface |
| `createBotChatState()` | factory → `BotChatState` |

Methods: `isLocallyBusy(target)`, `beginProcessing(target, prompt)`, `endProcessing(target)`, `beginSwitching(target)`, `endSwitching(target)`, `beginTranscribing(target)`, `endTranscribing(target)`, `getLastPrompt(target)`, `clearPromptMemory(target)`.

Uses closure-captured `Set<string>` for processing/switching/transcribing states and `Map<string, string>` for last prompts. Target is always `PiSessionContext`.

### `src/bot/chat-task-runner.ts` — Async Task Execution

| Export | Kind |
|--------|------|
| `ChatTaskRunner` | interface |
| `createChatTaskRunner(deps)` | factory → `ChatTaskRunner` |

Single method: `tryStartPrompt(target, promptText, task) → "started" | "busy"`. Wraps prompt execution with busy-guard logic — tracks running contexts via `Set<string>`, calls `beginProcessing`/`endProcessing` callbacks, catches task errors via `onTaskError`.

### `src/bot/prompt-handler.ts` — Prompt Flow Orchestration (~534 lines)

| Export | Kind |
|--------|------|
| `HandleUserPrompt` | type — `(ctx, target, userText, preloadedSlashCommands?, images?) → Promise<boolean>` |
| `createPromptHandler(options)` | factory → `HandleUserPrompt` |

Handles the full prompt lifecycle: check busy → mark busy → stream typing → create response message → debounced edit on text deltas → tool verbosity rendering (summary/all/errors-only/none) → finalize with abort keyboard removal → mark idle. Supports image analysis and extension dialog integration (`select`, `confirm`, `input`).

Deps: `chat-state`, `chat-task-runner`, `telegram-transport`, `message-rendering`, `telegram-ui-context`, `extension-dialogs`, `pi-session`, `config`.

### `src/bot/telegram-transport.ts` — Safe Telegram API Wrappers

| Export | Kind |
|--------|------|
| `TextOptions` | type — `{ parseMode?, fallbackText?, replyMarkup? }` |
| `getTelegramTarget(ctx)` | extract `PiSessionContext` from context |
| `safeReply(ctx, text, options?, target?)` | safe reply with split + parse-mode fallback |
| `sendTextMessage(api, target, text, options?)` | low-level send, returns `{ message_id }` |
| `safeEditMessage(bot, target, messageId, text, options?)` | edit with not-modified + parse fallback |
| `sendChatAction(api, target, action)` | typing indicator |
| `downloadTelegramFile(api, token, fileId, options?)` | download to temp dir (max 25 MB) |

`downloadTelegramFile` options: `maxFileSizeBytes`, `fileKind` (label for errors), `tempFilePrefix`. Returns temp file path. Handles rate limits via Grammy.

### `src/bot/message-rendering.ts` — HTML Formatting, Chunking & Rendering (~612 lines)

| Export | Kind |
|--------|------|
| `TelegramParseMode` | type — `"HTML"` |
| `RenderedText` | type — `{ text, fallbackText, parseMode? }` |
| `RenderedChunk` | type extends `RenderedText` + `{ sourceText }` |
| `TELEGRAM_MESSAGE_LIMIT` | const `4000` |
| `TOOL_OUTPUT_PREVIEW_LIMIT` | const `500` |
| `renderHelpPlain(info)` | plain-text help with session info |
| `renderHelpHTML(info)` | HTML help with session info |
| `renderSessionInfoPlain(info)` | session details plain text |
| `renderSessionInfoHTML(info)` | session details HTML |
| `renderPromptFailure(text, error)` | combined accumulated text + error status |
| `renderFailedText(error)` | prefixed error `RenderedText` |
| `renderPrefixedError(prefix, error, multiline?)` | generic prefixed error |
| `renderExtensionNotice(message, type?)` | ℹ️/⚠️/❌ notice |
| `renderExtensionError(path, event, error)` | extension error display |
| `renderToolStartMessage(toolName)` | tool start indicator |
| `renderToolEndMessage(toolName, output, isError)` | tool end with output preview |
| `formatToolSummaryLine(toolCounts)` | tool usage summary line |
| `splitMarkdownForTelegram(text)` | chunk at 4000 chars, newline-aware |
| `renderMarkdownChunkWithinLimit(markdown)` | single chunk with fallback |
| `formatMarkdownMessage(markdown)` | markdown → Telegram HTML |
| `buildStreamingPreview(text)` | truncate to 3800 chars for live preview |
| `appendWithCap(base, addition, cap)` | append with max length |
| `summarizeToolOutput(text)` | tail 500 chars with ellipsis |
| `findPreferredSplitIndex(text, maxLength)` | newline > space > hard cut |
| `trimLine(text, maxLength)` | single-line collapse + truncate |
| `stripHtml(text)` | remove HTML tags |
| `getWorkspaceShortName(workspace)` | last path segment |
| `isMessageNotModifiedError(error)` | Telegram error detection |
| `isTelegramParseError(error)` | Telegram parse error detection |
| `renderDialogPanel(title, bodyLines)` | box-drawing dialog panel |

### `src/bot/keyboard.ts` — Inline Keyboard Pagination

| Export | Kind |
|--------|------|
| `KeyboardItem` | type `{ label, callbackData }` |
| `PaginatedKeyboard` | interface `{ keyboard }` |
| `KEYBOARD_PAGE_SIZE` | const `6` |
| `NOOP_PAGE_CALLBACK_DATA` | re-export from `callback-data.js` |
| `paginateKeyboard(items, page, prefix)` | paginated inline keyboard with ◀️/▶️ |
| `appendKeyboardItems(keyboard, items)` | append items to existing keyboard |

### `src/bot/slash-command.ts` — Command Definitions & Normalization (~343 lines)

| Export | Kind |
|--------|------|
| `TELEPI_BOT_COMMANDS` | const array of 14 command descriptors |
| `TELEPI_LOCAL_COMMAND_NAMES` | `Set<string>` (bot commands + `"switch"`) |
| `NormalizedSlashCommand` | type `{ name, text }` |
| `CommandPickerFilter` | type `"all" \| "telepi" \| "pi"` |
| `CommandPickerEntry` | discriminated union `{ kind: "telepi" \| "pi", ... }` |
| `TelepiNativeCommandMenu` | type for native menu entries |
| `normalizeSlashCommand(text, botUsername?)` | parse raw text into command structure |
| `buildCommandPickerEntries(slashCommands)` | build picker entries from TelePi + Pi commands |
| `filterCommandPickerEntries(entries, filter)` | filter by kind |
| `getCommandPickerCounts(entries)` | count per filter |
| `getCommandPickerFilterName(filter)` | human-readable filter name |
| `getTelepiNativeCommandMenu(command, slashCommands)` | detect native menu from integrations |
| `rewriteSlashCommandForTelegram(command, slashCommands)` | passthrough rewrite |
| `buildChatScopedCommands(slashCommands)` | Telegram bot command list (max 100) |
| `buildChatScopedCommandSignature(commands)` | JSON signature for change detection |

`normalizeSlashCommand` handles `@bot` addressing and rejects commands addressed to other bots.

### `src/bot/extension-dialogs.ts` — Telegram-Backed Extension UI (~466 lines)

| Export | Kind |
|--------|------|
| `PendingExtensionDialog` | discriminated union `{ kind: "select" \| "confirm" \| "input", ... }` |
| `DialogCallbackResult` | type `{ callbackText, afterAnswer? }` |
| `ExtensionDialogManager` | interface |
| `createExtensionDialogManager(deps)` | factory → `ExtensionDialogManager` |

Methods: `hasPending(target)`, `getPendingKind(target)`, `openSelect(target, title, options, dialogOptions?)`, `openConfirm(target, title, message, dialogOptions?)`, `openInput(target, placeholder?, dialogOptions?)`, `consumeInput(target, userText)`, `cancelPending(target)`, `resolveSelect(target, dialogId, messageId, optionIndex)`, `resolveConfirm(target, dialogId, messageId, confirmed)`, `resolveCancel(target, dialogId, messageId)`.

Dialogs support AbortSignal and timeout. Renders box-drawing panels via `renderDialogPanel`.

### `src/bot/callback-query-logging.ts` — Callback Query Error Handling

| Export | Kind |
|--------|------|
| `COMMAND_MENU_CALLBACK_PREFIX` | const `"cmdm_"` |
| `isStaleCallbackQueryError(error)` | detect "query too old" / "query_id_invalid" |
| `describeCallbackQuerySource(callbackData)` | map callback data prefix to source label |
| `logCallbackQueryError(ctx, error, options?)` | log with stale suppression (30s window) |
| `resetCallbackQueryLogStateForTests()` | clear state for testing |

`describeCallbackQuerySource` recognizes prefixes: `cmdm_`, `ui_sel_`, `ui_cfm_`, `ui_x_`, `tree_`, `switch_`, `newws_`, `model_`, `cmd_`, `pi_abort`, `noop_page`.

### `src/bot/prompt-inbox.ts` — Filesystem Prompt Inbox Polling

| Export | Kind |
|--------|------|
| `PromptInboxPollResult` | type `"busy" \| "empty" \| "queued"` |
| `PromptInboxPollOptions` | interface — `{ inboxDir, target, isBusy, handlePrompt }` |
| `PromptInboxPollingOptions` | extends PollOptions + `{ intervalMs, onError? }` |
| `ClaimedPromptInboxFile` | interface `{ path, prompt, ack }` |
| `startPromptInboxPolling(options)` | start interval polling, returns stop function |
| `pollPromptInboxOnce(options)` | single poll cycle |
| `claimNextPromptInboxFile(inboxDir)` | claim oldest `.txt` file |

Polls an inbox directory for `.txt` files sorted by mtime. Claims file by reading content, returns `ack` callback that deletes the file. Skips empty files. Returns stop function to clear the interval timer.

### `src/bot/commands/` — Grouped Command Handlers

| File | Commands | Notes |
|------|----------|-------|
| `basic.ts` | `/start`, `/help`, `/new` | Welcome, session creation |
| `sessions.ts` | `/sessions`, `/session` | List/switch sessions |
| `model.ts` | `/model` | Model picker with inline keyboard |
| `tree.ts` | `/tree`, `/branch`, `/label` | Session tree navigation |
| `context.ts` | `/context` | Context window usage |

Note: `command-picker.ts` and `tree-callbacks.ts` are now in `src/bot/` (not `src/bot/commands/`).

---

## Pi Session Layer

### `src/pi-session.ts` — Pi SDK Session Wrapper (~1528 lines)

Core session management. Wraps the Pi SDK's `SessionManager` and `AgentSession`.

| Export | Kind |
|--------|------|
| `PiSessionContext` | interface `{ chatId, messageThreadId? }` |
| `PiSessionInfo` | interface `{ sessionId, sessionFile?, workspace, sessionName?, model?, modelFallbackMessage?, diagnostics? }` |
| `PiSessionDiagnostic` | interface `{ type: "info" \| "warning" \| "error", message }` |
| `PiSessionModelOption` | interface `{ provider, id, name, current, thinkingLevel? }` |
| `PiSessionSwitchResult` | extends `PiSessionInfo` + `{ cancelled }` |
| `PiSessionNewSessionOptions` | extends SDK options + `{ workspace? }` |
| `PiSessionSwitchOptions` | `{ withSession?, workspace? }` |
| `PiSessionForkOptions` | SDK fork options passthrough |
| `PiSessionCallbacks` | interface — streaming callbacks |
| `ResolvedSessionReference` | interface `{ id, path, cwd?, workspaceWarning?, matchType }` |
| `PiSessionService` | class — per-session operations |
| `PiSessionRegistry` | class — session store with `getOrCreate()` |
| `getPiSessionContextKey(ctx)` | string key for chat/topic |
| `consumeBootstrapSessionPath()` | one-shot bootstrap path consumption |

Key behaviors:
- **Bootstrap path**: `PI_SESSION_PATH` consumed by first `getOrCreate()` call, then ignored
- **Session storage**: `~/.pi/agent/sessions/<encoded-workspace>/`
- **Workspace switching**: re-scans coding tools via `createCodingTools(workspace)`
- **Self-management block**: blocks `launchctl` commands targeting `com.telepi` from inside sessions
- **Default bash timeout**: 120 seconds (`DEFAULT_BASH_TIMEOUT_SECONDS`) to prevent headless hangs
- **Provider response notices**: injects `createProviderResponseNoticeExtension()` for HTTP error surfacing
- **Path resolution**: delegates to `pi-session-paths.ts` for Docker remapping and session header parsing

### `src/pi-session-paths.ts` — Session Path Resolution

| Export | Kind |
|--------|------|
| `resolveSessionPathForRuntime(sessionPath)` | expand `~`, remap host paths to container paths |
| `readSessionHeader(sessionPath)` | read first-line JSON header `{ id, cwd? }` |
| `resolveWorkspacePathForRuntime(workspace)` | verify workspace exists |

Handles Docker path remapping: detects `.pi/agent/` marker in path and tries `/home/telepi/.pi/agent/` and `/root/.pi/agent/` as container bases. Uses sync I/O for header reading (first 1024 bytes only).

### `src/model-scope.ts` — Model Filtering & Scoping

| Export | Kind |
|--------|------|
| `ScopedModelOption` | interface `{ model, thinkingLevel? }` |
| `resolveScopedModels(settingsManager, modelRegistry)` | filter/group available models from settings patterns |
| `resolveInitialScopedModelSelection(options)` | pick model for new session based on config + defaults |

Supports glob patterns (`*`, `?`, `[`) via `minimatch`, thinking level suffixes (`:high`, `:low`), partial model name matching with alias preference (`-latest` suffix).

---

## Install / Service Layer

### `src/install.ts` — Public Facade

Re-exports setup and status functions. Orchestrates config → service unit → extension install pipeline.

| Export | Kind |
|--------|------|
| `getTelePiStatus(cliModuleUrl)` | → `TelePiStatus` — version, config, service, extension |
| `setupTelePi(cliModuleUrl, options?)` | → `TelePiSetupResult` — full install/upgrade |
| `resolveTelePiInstallContext(cliModuleUrl)` | re-export from platform |
| `ensureTelePiConfig(context, options?)` | re-export from config |
| `buildLaunchAgentPlist(context)` | re-export from launchd |
| `detectPlatform()` | re-export from platform |
| Type re-exports | `TelePiInstallContext`, `TelePiSetupResult`, `TelePiStatus`, etc. |

### `src/install/platform.ts` — Platform Detection

Platform detection (`darwin` → macOS, `linux` → Linux), install context resolution. Exports `resolveTelePiInstallContext(cliModuleUrl)` and `detectPlatform()`. Provides `getPlatformInstallHint(tool)` for installation guidance.

### `src/install/config.ts` — Config File Management

Creates/updates `~/.config/telepi/config.env`. Preserves existing optional values on re-setup. Exports `ensureTelePiConfig(context, options?)` and `getServiceConfigSource(context)`.

### `src/install/launchd.ts` — macOS LaunchAgent

Generates plist from template, installs to `~/Library/LaunchAgents/com.telepi.plist`, manages via `launchctl`. Exports `buildLaunchAgentPlist(context)` and `getInstalledConfigStatus(context)`.

### `src/install/systemd.ts` — Linux systemd User Service

Generates unit file from template, installs to `~/.config/systemd/user/telepi.service`, manages via `systemctl --user`.

### `src/install/service-manager.ts` — Shared Service Interface

Common interface for launchd/systemd operations (install, uninstall, restart, status). Platform-agnostic `ServiceManager` abstraction.

### `src/install/extension.ts` — Pi Extension Install

Symlinks `extensions/telepi-handoff.ts` to `~/.pi/agent/extensions/`. Exports `installExtension(context)` and `getExtensionStatus(context)`.

### `src/install/clipboard.ts` — Cross-Platform Clipboard

`copyToClipboard(text)` — tries `pbcopy` (macOS), `wl-copy`, `xclip`, `xsel` (Linux).

### `src/install/shared.ts` — Shared Types/Constants

Common types and constants used across install modules. Exports `TelePiInstallContext`, `TelePiSetupResult`, `TelePiStatus`, `ServiceStatus`, `ExtensionStatus`, `ExtensionInstallMode`, `TelePiConfigSetupResult`, `TelePiConfigSetupValues`, `TelePiStatusConfigSource`, `TELEPI_LAUNCHD_LABEL`, etc.

---

## Voice Layer

### `src/voice.ts` — Audio Transcription (~539 lines)

| Export | Kind |
|--------|------|
| `TranscriptionResult` | interface `{ text, backend, durationMs }` |
| `TranscriptionBackend` | type `"parakeet" \| "sherpa-onnx" \| "openai"` |
| `VoiceBackendStatus` | interface `{ backends, warning? }` |
| `getAvailableBackends()` | checks which backends are available |
| `getVoiceBackendStatus()` | backends + misconfiguration warnings |
| `transcribeAudio(filePath)` | tries best available backend |

Fallback chain: Parakeet CoreML → Sherpa-ONNX → OpenAI Whisper. Downloads voice to temp dir via ffmpeg, transcribes, deletes immediately.

- **Parakeet CoreML**: Apple Silicon native, mutex-serialized (`_parakeetMutex`)
- **Sherpa-ONNX**: cross-platform offline, requires `SHERPA_ONNX_MODEL_DIR` env, mutex-serialized (`_sherpaMutex`), reuses single recognizer instance
- **OpenAI Whisper**: cloud fallback, requires `OPENAI_API_KEY`
- **Thread safety**: both native backends use promise-based mutex to prevent concurrent engine access across overlapping voice notes

---

## Provider Notices

### `src/provider-response-notices.ts` — Provider HTTP Error Notices

| Export | Kind |
|--------|------|
| `ProviderResponseNoticeEvent` | interface `{ status, headers }` |
| `ProviderResponseNotice` | interface `{ message, type }` |
| `getProviderResponseNotice(event)` | map HTTP status to user-facing notice |
| `createProviderResponseNoticeExtension()` | `ExtensionFactory` — hooks `after_provider_response` |

Maps provider HTTP errors to actionable notices:
- 401/403 → error (auth failure)
- 429 → warning (rate limit, with Retry-After)
- 408/5xx → warning (unavailable, with Retry-After)
- Other non-2xx → warning if Warning/Retry-After headers present

Extracts `request-id`, `x-request-id`, `anthropic-request-id`, `openai-request-id` headers.

---

## Utilities

### `src/config.ts` — Configuration Loader

Hand-rolled `.env` parser (no dotenv dependency). Supports `export KEY=VALUE` syntax.

| Export | Kind |
|--------|------|
| `TelePiConfig` | interface — `{ telegramBotToken, telegramAllowedUserIds, telegramAllowedUserIdSet, workspace, piSessionPath?, piModel?, toolVerbosity, promptInboxDir?, promptInboxIntervalMs }` |
| `TelePiConfigPathInfo` | interface — `{ explicitPath?, defaultPath, localPath, resolvedPath?, source }` |
| `TelePiConfigPathSource` | type `"explicit" \| "default" \| "cwd" \| "missing"` |
| `ToolVerbosity` | type `"all" \| "summary" \| "errors-only" \| "none"` |
| `loadConfig()` | reads config from env → `.env` (cwd) → `~/.config/telepi/config.env` |
| `getConfigEnvPathInfo()` | returns config path resolution details |
| `parseAllowedUserIds(raw)` | parse comma-separated user IDs |

Config resolution priority: `TELEPI_CONFIG` env → `.env` in cwd → `~/.config/telepi/config.env`. Workspace: `/workspace` in Docker → `TELEPI_WORKSPACE` → `process.cwd()`.

### `src/errors.ts` — Error Helpers

| Export | Kind |
|--------|------|
| `formatError(error)` | extract message string for internal logging |
| `toFriendlyError(error)` | user-facing error messages, strips internal prefixes |

No custom error classes. Plain `Error` + string matching. Recognizes: abort, session not initialized, model not found, Telegram file errors, network errors.

### `src/format.ts` — Markdown → Telegram HTML

Converts markdown to Telegram HTML. Pipeline: escape HTML → extract code blocks → extract inline code → bold → italic → links → blockquotes → restore placeholders. Uses Unicode private-use-area characters (`\uE000`, `\uE001`) for placeholder protection.

Exports: `escapeHTML(text)`, `formatTelegramHTML(markdown)`.

### `src/tree.ts` — Session Tree Rendering (~671 lines)

| Export | Kind |
|--------|------|
| `SessionTreeNodeLike` | interface `{ entry, children, label? }` |
| `TreeFilterMode` | type `"default" \| "user-only" \| "all-with-buttons"` |
| `TreeRenderResult` | interface `{ text, buttons, totalEntries, shownEntries, page, totalPages }` |
| `TreeButton` | interface `{ label, callbackData }` |
| `renderTree(nodes, options)` | tree rendering with pagination and filtering |
| `renderBranchConfirmation(node, labelsMap)` | branch confirmation text |
| `describeEntry(entry)` | human-readable entry description |
| `truncateText(text, maxLength)` | truncate with ellipsis |

Tree uses box-drawing characters for structure, paginated at 10 entries per page with inline keyboard buttons.

### `src/callback-data.ts` — Callback Data Constants

Single export: `NOOP_PAGE_CALLBACK_DATA = "noop_page"`. Used for pagination page indicator buttons that should not trigger any action.

### `src/telegram-ui-context.ts` — Extension UI Adapter

| Export | Kind |
|--------|------|
| `TelegramExtensionNoticeType` | type `"info" \| "warning" \| "error"` |
| `CreateTelegramUIContextOptions` | interface — `{ notify, select?, confirm?, input? }` |
| `createTelegramUIContext(options)` | → `ExtensionUIContext` |

Adapts Pi extension UI calls to Telegram dialogs. Provides a plain-text theme shim (no ANSI rendering). Bridges `select`, `confirm`, `input` to inline keyboards and message waiting. Throws on unsupported methods (`custom`, `editor`).

### `src/paths.ts` — Path Utilities

| Export | Kind |
|--------|------|
| `DOCKER_WORKSPACE_PATH` | const `"/workspace"` |
| `getHomeDirectory()` | `process.env.HOME` || `os.homedir()` |
| `expandHomePath(filePath)` | `~/...` → absolute path |
| `resolvePathFromCwd(filePath, cwd?)` | expand home + resolve relative |
| `getDefaultTelePiConfigPath(home?)` | `~/.config/telepi/config.env` |
| `getDefaultSystemdUserDir(home?)` | `~/.config/systemd/user` |
| `getDefaultLogDir(home?, platform)` | macOS: `~/Library/Logs/TelePi`, Linux: `~/.local/state/telepi/logs` |

### `src/entrypoint.ts` — ESM Entrypoint Guard

`isEntrypoint(moduleUrl, argvPath?)` — ESM equivalent of `if (require.main === module)`. Uses `realpathSync` + `fileURLToPath` comparison.

---

## Orchestration Entrypoints

### `src/index.ts` — Bot Startup

Creates `PiSessionRegistry`, `Bot`, registers commands. Graceful shutdown on SIGINT/SIGTERM with 500ms timeout for session disposal. Polling restart with backoff (5 attempts, 3s delay on 409 Conflict).

Exports: `startBot()`.

### `src/cli.ts` — CLI Commands

| Command | Description |
|---------|-------------|
| `telepi` / `telepi start` | Start the bot (default) |
| `telepi setup` | Interactive setup (TTY only) |
| `telepi setup <token> <userids> <workspace>` | Non-interactive fast setup |
| `telepi status` | Show installed-mode status |
| `telepi version` / `--version` / `-v` | Print TelePi version |
| `telepi help` / `--help` / `-h` | Show usage help |

Uses switch/case dispatch. Errors print to stderr with `telepi:` prefix and exit code 1.

---

*Updated from TypeScript source on 2026-06-04.*
