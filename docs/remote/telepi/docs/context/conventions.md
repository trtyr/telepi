# Conventions

## TypeScript Configuration

- **Strict mode** is enabled (`tsconfig.json` `"strict": true`). No `any` escape hatches.
- **`verbatimModuleSyntax`** is on — type-only imports must use `import type { ... }` or the inline `import { type Foo, Bar }` syntax. Mixing value and type-only imports in a single statement is fine using the inline form.
- **ESM only** — `"type": "module"` in package.json, `"module": "Node16"` / `"moduleResolution": "Node16"` in tsconfig.
- **Node 20+** required (`"engines": { "node": ">=20" }`).
- **Target ES2022** — top-level await, `Array.at()`, structured clone, etc. are available without polyfills.

## Import Rules

- All local imports use the `.js` suffix, even for `.ts` source files. This is mandatory for Node16 module resolution:
  ```ts
  import { loadConfig } from "./config.js";
  import type { TelePiConfig } from "./config.js";
  ```
- Node built-ins use the `node:` protocol prefix:
  ```ts
  import { existsSync } from "node:fs";
  import path from "node:path";
  ```
- No barrel files (`index.ts` re-exports). Each module is imported directly by path.
- Third-party types are imported inline:
  ```ts
  import { InlineKeyboard, Bot, type Context } from "grammy";
  ```

## Exports

- **Named exports only** for library modules. Default exports are used only for the single extension entrypoint (`extensions/telepi-handoff.ts`).
- Facade modules (e.g. `src/install.ts`) re-export types with `export type { ... }` to maintain a clean public surface.

## Naming

- **Files**: lowercase with hyphens for multi-word names (`pi-session.ts`, `model-scope.ts`, `chat-task-runner.ts`). No `camelCase` filenames.
- **Types/Interfaces**: PascalCase (`TelePiConfig`, `PiSessionContext`, `ToolVerbosity`).
- **Functions**: camelCase (`loadConfig`, `formatError`, `toFriendlyError`).
- **Constants**: UPPER_SNAKE_CASE for true constants (`DOCKER_WORKSPACE_PATH`, `DEFAULT_PROMPT_INBOX_INTERVAL_MS`, `SAFE_URL_PROTOCOL`). camelCase for local `const` bindings that are scoped to a function.
- **String union types** for enums of config options rather than TypeScript `enum`:
  ```ts
  export type ToolVerbosity = "all" | "summary" | "errors-only" | "none";
  ```

## Style

- 2-space indentation, double quotes, semicolons.
- No linter config (eslint, prettier) in the repo — style is enforced by convention and review.

## Error Handling

- Errors shown to Telegram users go through `toFriendlyError()` (`src/errors.ts`) which maps raw error messages to human-readable strings and strips internal prefixes like `"Pi session prompt failed:"`.
- **Dual-render pattern**: `src/bot/message-rendering.ts` renders every user-facing message as a `RenderedText` object with `text` (HTML), `fallbackText` (plain), and `parseMode`. The transport layer (`sendTextMessage`, `safeReply`, `safeEditMessage`) tries HTML first and falls back to plain text if Telegram returns a parse error.
  ```ts
  export type RenderedText = {
    text: string;
    fallbackText: string;
    parseMode?: "HTML";
  };
  ```
- Internal error logging uses `console.error` with a descriptive prefix string:
  ```ts
  console.error("Failed to dispose session after setup error:", disposeError);
  console.error(`Failed to send tool start message for ${toolName}`, error);
  ```
- Validation errors in config throw immediately with descriptive messages:
  ```ts
  throw new Error(`Missing required environment variable: ${name}`);
  throw new Error(`Invalid Telegram user id in TELEGRAM_ALLOWED_USER_IDS: ${value}`);
  ```
- Config parsing warns on invalid optional values and falls back to defaults rather than crashing:
  ```ts
  console.warn(`Invalid TELEPI_PROMPT_INBOX_INTERVAL_MS value: "${raw}". Falling back to ...`);
  ```

## Configuration Pattern

- `loadConfig()` in `src/config.ts` is the single source of truth. It reads `.env` files manually (no dotenv dependency), resolves workspace paths, and returns a typed `TelePiConfig` interface.
- Required env vars use `requireEnv()` which throws. Optional values use `optionalString()` which returns `undefined` for empty/whitespace strings.
- Numeric config values are clamped or fall back to defaults with a `console.warn`, never silently swallowed.

## Testing

- Vitest with `globals: true` — no need to import `describe`, `it`, `expect`.
- Test files live in `test/`, mirroring `src/` structure: `test/bot.test.ts`, `test/bot/*.test.ts`.
- Coverage thresholds are enforced: 85% lines/functions/statements, 75% branches.
- `src/index.ts` and `src/install.ts` are excluded from coverage (orchestration facades).
- No `@ts-ignore`, `@ts-expect-error`, or `eslint-disable` comments exist in `src/`.

## Anti-Patterns to Avoid

- **No `export default`** in library modules — makes tree-shaking harder and conflicts with `verbatimModuleSyntax`.
- **No barrel re-export files** — import the specific module directly.
- **No dotenv** dependency — `.env` parsing is hand-rolled in `src/config.ts` to support `export` prefixed lines and custom resolution logic.
- **No `any` casts** — strict mode is on; use `unknown` and narrow.
- **No `console.log` in library code** — `console.log` is reserved for CLI output in `src/cli.ts` and startup messages in `src/index.ts`. Everything else uses `console.error` or `console.warn`.

## Formatting Pipeline

Telegram message rendering follows a specific order in `src/format.ts`: escape HTML → extract code blocks → extract inline code → bold → italic → links → blockquotes → restore placeholders. This order matters — the placeholder system uses Unicode private-use-area characters to protect code spans from markdown transformation.

The actual Telegram message size limit enforced in code is **4000 characters** (`TELEGRAM_MESSAGE_LIMIT` in `src/bot/message-rendering.ts`), not 4096. This provides a safety margin. Message splitting in `splitTelegramText()` prefers newline boundaries, then space boundaries, then hard-cuts.

For streaming markdown output, `splitMarkdownForTelegram()` targets a smaller **3000-character chunk size** (`FORMATTED_CHUNK_TARGET`) and renders each chunk through `formatTelegramHTML()` independently, because the placeholder system does not survive chunk boundaries.

## Commit Style

Conventional Commits with optional scopes: `fix: support switching TelePi sessions by id`, `feat(docker): allow user npm global installs`.
