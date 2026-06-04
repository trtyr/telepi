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
| `/switch` | Alias for `/sessions` | Same as `/sessions` |
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

interface PiSessionNewSessionOptions {
  workspace?: string;
  // Inherits parentSession, setup, withSession from SDK runtime options
}

type PiSessionSwitchOptions = {
  workspace?: string;
  withSession?: unknown;
};

type PiSessionForkOptions = NonNullable<Parameters<AgentSessionRuntime["fork"]>[1]>;
```

**Key classes:**

```typescript
// Manages a single per-chat Pi session
class PiSessionService {
  static create(config: TelePiConfig): Promise<PiSessionService>;
  getInfo(): PiSessionInfo;
  prompt(text: string, images?: ImageContent[]): Promise<void>;
  abort(): Promise<void>;
  newSession(request?: string | PiSessionNewSessionOptions): Promise<{ info: PiSessionInfo; created: boolean }>;
  switchSession(sessionPath: string, request?: string | PiSessionSwitchOptions): Promise<PiSessionSwitchResult>;
  fork(entryId: string, options?: PiSessionForkOptions): Promise<{ cancelled: boolean }>;
  listModels(showAll?: boolean): Promise<PiSessionModelOption[]>;
  setModel(provider: string, modelId: string, thinkingLevel?: ThinkingLevel): Promise<string>;
  listAllSessions(): Promise<Array<{ id: string; firstMessage: string; path: string; messageCount: number; cwd: string; modified: Date; name?: string }>>;
  listWorkspaces(): Promise<string[]>;
  resolveSessionReference(sessionReference: string): Promise<ResolvedSessionReference>;
  handback(): Promise<{ sessionFile?: string; workspace: string }>;
  getContextUsage(): ContextUsage | undefined;
  getSessionStats(): SessionStats | undefined;
  getTree(): SessionTreeNodeLike[];
  getLeafId(): string | null;
  getEntry(id: string): SessionEntry | undefined;
  getChildren(id: string): SessionEntry[];
  navigateTree(targetId: string, options?: { summarize?: boolean; customInstructions?: string; replaceInstructions?: boolean; label?: string }): Promise<{ editorText?: string; cancelled: boolean }>;
  setLabel(targetId: string, label: string): void;
  getLabels(): Array<{ id: string; label: string; description: string }>;
  subscribe(callbacks: PiSessionCallbacks): () => void;
  dispose(): void;
}

// Registry of per-chat PiSessionService instances
class PiSessionRegistry {
  static create(config: TelePiConfig): Promise<PiSessionRegistry>;
  has(context: PiSessionContext): boolean;
  get(context: PiSessionContext): PiSessionService | undefined;
  getInfo(context: PiSessionContext): PiSessionInfo;
  getOrCreate(context: PiSessionContext): Promise<PiSessionService>;
  remove(context: PiSessionContext): void;
  dispose(): void;
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

interface VoiceBackendStatus {
  backends: TranscriptionBackend[];
  warning?: string;
}
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
type CommandPickerEntry =
  | { id: number; kind: "telepi"; command: string; description: string; label: string; commandText: string }
  | { id: number; kind: "pi"; name: string; description: string; label: string; commandText: string; source: string };
type TelepiNativeCommandMenuEntry = { id: string; label: string; commandText: string };
type TelepiNativeCommandMenu = { name: string; bareCommandText: string; title: string; entries: TelepiNativeCommandMenuEntry[] };

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

// src/bot/extension-dialogs.ts
type PendingExtensionDialog =
  | { kind: "select"; dialogId: string; messageId: number; title: string; options: string[]; resolve: (value: string | undefined) => void; }
  | { kind: "confirm"; dialogId: string; messageId: number; title: string; message: string; resolve: (value: boolean) => void; }
  | { kind: "input"; dialogId: string; messageId: number; title: string; placeholder?: string; resolve: (value: string | undefined) => void; };

interface ExtensionDialogManager {
  hasPending(target: PiSessionContext): boolean;
  openSelect(target: PiSessionContext, title: string, options: string[], ...): Promise<string | undefined>;
  openConfirm(target: PiSessionContext, title: string, message: string, ...): Promise<boolean>;
  openInput(target: PiSessionContext, title: string, placeholder?: string, ...): Promise<string | undefined>;
  cancelPending(target: PiSessionContext): Promise<boolean>;
}

// src/bot/message-rendering.ts
type TelegramParseMode = "HTML";
type RenderedText = { text: string; fallbackText: string; parseMode?: TelegramParseMode };
type RenderedChunk = RenderedText & { sourceText: string };
```

### Model Scoping (`src/model-scope.ts`)

```typescript
interface ScopedModelOption {
  model: Model<Api>;
  thinkingLevel?: ThinkingLevel;
}
```

### Callback Data (`src/callback-data.ts`)

```typescript
const NOOP_PAGE_CALLBACK_DATA = "noop_page";
```
