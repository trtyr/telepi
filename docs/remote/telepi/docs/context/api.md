# TelePi API Reference

## CLI Commands

TelePi exposes a CLI via `telepi` (or `node dist/cli.js`).

| Command | Description | Parameters |
|---|---|---|
| `telepi` / `telepi start` | Start the Telegram bot | None |
| `telepi setup` | Interactive setup (TTY) or fast setup | `[<bot_token> <userids> <workspace>]` |
| `telepi status` | Show installed-mode status (config, service, extension) | None |
| `telepi version` / `--version` / `-v` | Print the TelePi version | None |
| `telepi help` / `--help` / `-h` | Show usage help | None |

**Source:** `src/cli.ts`

---

## Telegram Bot Commands

All commands are slash-commands registered via the Telegram Bot API. Each chat/topic gets its own independent Pi session.

| Command | Description | Parameters |
|---|---|---|
| `/start` | Welcome message, session info, voice support status | None |
| `/help` | List all commands and usage tips | None |
| `/commands` | Open the interactive command picker (TelePi + Pi commands) | None |
| `/new` | Create a new session. If multiple workspaces exist, shows a picker | None |
| `/retry` | Re-send the last prompt in this chat/topic | None |
| `/handback` | Hand the active session back to Pi CLI (provides terminal command) | None |
| `/abort` | Cancel the currently running operation | None |
| `/session` | Show current session details (model, workspace, ID) | None |
| `/sessions` | List all saved sessions with switch picker | `/sessions <path\|id>` to switch directly |
| `/context` | Show context window usage and session token/cost stats | None |
| `/model` | Open the model picker to switch the AI model | None |
| `/tree` | View the conversation tree | `/tree all` or `/tree user` for filter modes |
| `/branch` | Navigate to a tree entry | `/branch <entry-id>` |
| `/label` | Label a tree entry | `/label <name>` (current leaf), `/label <id> <name>`, `/label clear <id>` |

Additionally, Pi-native slash commands (from prompts, skills, and extensions) are synced as chat-scoped Telegram commands at runtime.

**Source:** `src/bot/slash-command.ts` (L10-25), `src/bot/commands/`

---

## Telegram Callback Queries

Inline keyboard interactions use callback query patterns. Each prefix maps to a handler module.

| Callback Pattern | Source | Description |
|---|---|---|
| `switch_<idx>` | sessions | Switch to a session from the picker |
| `newws_<idx>` | sessions | Select workspace for new session |
| `model_<idx>` | model | Select a model from the picker |
| `model_show_all` | model | Toggle to show all models (vs. scoped) |
| `tree_page_<n>` | tree-callbacks | Paginate the tree view |
| `tree_nav_<entryId>` | tree-callbacks | Show branch confirmation for a tree entry |
| `tree_go_<entryId>` | tree-callbacks | Navigate to a tree entry (no summary) |
| `tree_sum_<entryId>` | tree-callbacks | Navigate to a tree entry with branch summary |
| `tree_cancel` | tree-callbacks | Cancel pending tree navigation |
| `tree_mode_<mode>` | tree-callbacks | Switch tree filter mode (default/all/user) |
| `cmd_pick_<id>` | command-picker | Execute a command from the picker |
| `cmd_page_<n>` | command-picker | Paginate the command picker |
| `cmd_filter_<f>` | command-picker | Filter commands (all/telepi/pi) |
| `cmdm_*` | command-menu | Native command menu entries |
| `ui_sel_*` | extension-dialogs | Extension select dialog option |
| `ui_cfm_*` | extension-dialogs | Extension confirm dialog |
| `ui_x_*` | extension-dialogs | Extension dialog cancel |
| `pi_abort` | bot | Abort current Pi operation |
| `noop_page` | keyboard | Pagination placeholder (no-op) |

**Sources:** `src/bot/tree-callbacks.ts`, `src/bot/command-picker.ts`, `src/bot/callback-query-logging.ts`

---

## Pi Extension API

TelePi registers as a Pi extension via `extensions/telepi-handoff.ts`.

**Registered command:** `handoff` — hands off the current Pi CLI session to TelePi (Telegram).

**Handoff modes** (controlled by `TELEPI_HANDOFF_MODE`):

| Mode | Behavior |
|---|---|
| `auto` | Auto-detect: prefers launchd (macOS) > systemd (Linux) > direct |
| `direct` | Start TelePi as a background process (global install or source checkout) |
| `launchd` | Restart TelePi via `launchctl kickstart` (macOS) |
| `systemd` | Restart TelePi via `systemctl --user restart` (Linux) |

**Key exports from the extension:**

```typescript
type HandoffMode = "direct" | "launchd" | "systemd";

type DirectLaunchTarget =
  | { kind: "installed"; homeDirectory: string; installedConfigPath: string }
  | { kind: "source"; telePiDir: string }
  | { kind: "unavailable"; reason: "missing-installed-config" | "missing-telepi"; installedConfigPath: string };
```

**Source:** `extensions/telepi-handoff.ts`

---

## Exported Types and Interfaces

### Configuration (`src/config.ts`)

```typescript
type ToolVerbosity = "all" | "summary" | "errors-only" | "none";

interface TelePiConfig {
  telegramBotToken: string;
  telegramAllowedUserIds: number[];
  telegramAllowedUserIdSet: Set<number>;
  workspace: string;
  piSessionPath?: string;
  piModel?: string;
  toolVerbosity: ToolVerbosity;
  promptInboxDir?: string;
  promptInboxIntervalMs: number;
}

type TelePiConfigPathSource = "explicit" | "default" | "cwd" | "missing";

interface TelePiConfigPathInfo {
  explicitPath?: string;
  defaultPath: string;
  localPath: string;
  resolvedPath?: string;
  source: TelePiConfigPathSource;
}
```

### Session Model (`src/pi-session.ts`)

```typescript
interface PiSessionContext {
  chatId: number | string;
  messageThreadId?: number;
}

interface PiSessionInfo {
  sessionId: string;
  sessionFile?: string;
  workspace: string;
  sessionName?: string;
  modelFallbackMessage?: string;
  model?: string;
  diagnostics?: PiSessionDiagnostic[];
}

interface PiSessionDiagnostic {
  type: "info" | "warning" | "error";
  message: string;
}

interface PiSessionSwitchResult extends PiSessionInfo {
  cancelled: boolean;
}

interface PiSessionModelOption {
  provider: string;
  id: string;
  name: string;
  current: boolean;
  thinkingLevel?: ThinkingLevel;
}

interface ResolvedSessionReference {
  id: string;
  path: string;
  cwd?: string;
  workspaceWarning?: string;
  matchType: "path" | "id" | "prefix";
}

interface PiSessionCallbacks {
  onTextDelta: (delta: string) => void;
  onToolStart: (toolName: string, toolCallId: string) => void;
  onToolUpdate: (toolCallId: string, partialResult: string) => void;
  onToolEnd: (toolCallId: string, isError: boolean) => void;
  onAgentEnd: () => void;
}
```

### Tree Model (`src/tree.ts`)

```typescript
interface SessionTreeNodeLike {
  entry: SessionEntry;
  children: SessionTreeNodeLike[];
  label?: string;
}

type TreeFilterMode = "default" | "user-only" | "all-with-buttons";

interface TreeButton {
  label: string;
  callbackData: string;
}

interface TreeRenderResult {
  text: string;
  buttons: TreeButton[];
  totalEntries: number;
  shownEntries: number;
  page: number;
  totalPages: number;
}
```

### Voice Transcription (`src/voice.ts`)

```typescript
interface TranscriptionResult {
  text: string;
  backend: "parakeet" | "sherpa-onnx" | "openai";
  durationMs: number;
}

type TranscriptionBackend = "parakeet" | "sherpa-onnx" | "openai";
```

### Telegram UI Context (`src/telegram-ui-context.ts`)

```typescript
type TelegramExtensionNoticeType = "info" | "warning" | "error";

interface CreateTelegramUIContextOptions {
  notify: (message: string, type?: TelegramExtensionNoticeType) => void;
  select?: (title: string, options: string[], dialogOptions?: { signal?: AbortSignal; timeout?: number }) => Promise<string | undefined>;
  confirm?: (title: string, message: string, dialogOptions?: { signal?: AbortSignal; timeout?: number }) => Promise<boolean>;
  input?: (title: string, placeholder?: string, dialogOptions?: { signal?: AbortSignal; timeout?: number }) => Promise<string | undefined>;
}
```

### Bot Internals (`src/bot/`)

```typescript
// src/bot/keyboard.ts
type KeyboardItem = { label: string; callbackData: string };

// src/bot/slash-command.ts
type NormalizedSlashCommand = { name: string; text: string };
type CommandPickerFilter = "all" | "telepi" | "pi";

// src/bot/message-rendering.ts
interface ContextUsageInfo {
  tokens: number | null;
  contextWindow: number;
  percent: number | null;
}

interface SessionStatsInfo {
  userMessages: number;
  assistantMessages: number;
  toolCalls: number;
  toolResults: number;
  totalMessages: number;
  tokens: { input: number; output: number; cacheRead: number; cacheWrite: number; total: number };
  cost: number;
  contextUsage?: ContextUsageInfo;
  sessionFile: string | undefined;
  sessionId: string;
}

// src/bot/chat-state.ts
interface BotChatState {
  isLocallyBusy(target: PiSessionContext): boolean;
  beginProcessing(target: PiSessionContext, promptText: string): void;
  endProcessing(target: PiSessionContext): void;
  beginSwitching(target: PiSessionContext): void;
  endSwitching(target: PiSessionContext): void;
  beginTranscribing(target: PiSessionContext): void;
  endTranscribing(target: PiSessionContext): void;
  getLastPrompt(target: PiSessionContext): string | undefined;
  clearPromptMemory(target: PiSessionContext): void;
}
```

### Model Scoping (`src/model-scope.ts`)

```typescript
interface ScopedModelOption {
  model: Model<Api>;
  thinkingLevel?: ThinkingLevel;
}
```
