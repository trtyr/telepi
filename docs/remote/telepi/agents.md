# TelePi — Agent Reference

> Telegram bridge for the Pi coding agent. Runs locally, bridges Pi sessions to Telegram, supports voice/image/extension dialogs, hands off between CLI and mobile.
>
> **Source:** https://github.com/benedict2310/TelePi
> **Stack:** TypeScript / Node.js 20+ / grammY / Pi SDK
> **npm:** `@futurelab-studio/telepi` v0.4.2

## Quick Reference

| What | Where |
|------|-------|
| Architecture | [docs/context/architecture.md](docs/context/architecture.md) |
| Modules | [docs/context/modules.md](docs/context/modules.md) |
| Tech Stack | [docs/context/tech-stack.md](docs/context/tech-stack.md) |
| Conventions | [docs/context/conventions.md](docs/context/conventions.md) |
| API | [docs/context/api.md](docs/context/api.md) |
| Deploy | [docs/context/deploy.md](docs/context/deploy.md) |

## Overview

TelePi wraps Pi's `AgentSessionRuntime` in a grammY-based Telegram bot. Each Telegram chat/topic gets its own isolated Pi session. Supports text, voice (local CoreML/Sherpa-ONNX or cloud Whisper), image prompts, and Pi extension UI dialogs via inline keyboards.

Key flow: Pi CLI `/handoff` → Telegram text/voice/image → `/handback` → resume in terminal.

## Architecture

```
Telegram API → grammY Bot Layer → Prompt Handler → PiSessionRegistry
    → PiSessionService → AgentSessionRuntime → LLM Providers
```

- **Entrypoints:** `src/index.ts` (bot), `src/cli.ts` (CLI), `src/entrypoint.ts` (ESM guard)
- **Bot layer:** `src/bot.ts` + `src/bot/*.ts` (12 files) — commands, callbacks, transport, state
- **Session layer:** `src/pi-session.ts` (~1528 lines) — wraps Pi SDK, per-chat isolation
- **Voice:** `src/voice.ts` — 3-tier fallback (parakeet → sherpa → openai)
- **Install:** `src/install.ts` + `src/install/*.ts` — platform-abstracted service management

→ [Full architecture](docs/context/architecture.md)

## Key Modules

| Area | Module | Responsibility |
|------|--------|----------------|
| Bot | `src/bot.ts` | Composition root, handler wiring |
| Bot | `src/bot/prompt-handler.ts` | Stream Pi responses to Telegram |
| Bot | `src/bot/telegram-transport.ts` | API primitives, rate-limit handling |
| Bot | `src/bot/commands/*.ts` | 5 command groups (basic, sessions, context, model, tree) |
| Session | `src/pi-session.ts` | Session lifecycle, registry, bash timeout patch |
| Session | `src/model-scope.ts` | Model glob resolution with thinking levels |
| Voice | `src/voice.ts` | Audio transcription with fallback chain |
| Install | `src/install/platform.ts` | macOS/Linux platform detection |
| Install | `src/install/service-manager.ts` | Abstract service interface |
| Config | `src/config.ts` | Env loading, workspace resolution |

→ [Full module reference](docs/context/modules.md)

## Tech Stack

| Component | Choice |
|-----------|--------|
| Language | TypeScript 5.7+, ESM, strict mode |
| Runtime | Node.js 20+ |
| Telegram | grammY ^1.35.0 + auto-retry |
| Pi Agent | `@mariozechner/pi-coding-agent` 0.70.2 (pinned) |
| Testing | Vitest 3.2+ (85% line coverage threshold) |
| Build | `tsc` (ES2022 target, Node16 module resolution) |
| Container | node:22-alpine, non-root, 2GB limit |
| Voice (optional) | parakeet-coreml (Apple Silicon), sherpa-onnx (cross-platform), OpenAI Whisper |

→ [Full tech stack](docs/context/tech-stack.md)

## Commands

### CLI

| Command | Purpose |
|---------|---------|
| `telepi setup` | Interactive config + service install |
| `telepi start` | Start the bot |
| `telepi status` | Show config, service, extension status |
| `telepi version` | Print version |

### Telegram Bot

| Command | Purpose |
|---------|---------|
| `/start` `/help` | Welcome / usage guide |
| `/new` | Create new session |
| `/sessions` | List/switch sessions |
| `/handback` | Resume session in CLI |
| `/abort` | Cancel running operation |
| `/retry` | Re-send last prompt |
| `/model` | Switch AI model |
| `/tree` `/branch` `/label` | Session tree navigation |
| `/context` | Context window usage |
| `/commands` | Interactive command picker |

→ [Full API reference](docs/context/api.md)

## Conventions

- **ESM only**, `.js` import suffixes mandatory, `node:` protocol for built-ins
- **Named exports only** (default exports only for extension entrypoint)
- **No barrel files** — import directly by path
- **No dotenv** — hand-rolled `.env` parsing in `src/config.ts`
- **2-space indent, double quotes, semicolons** (no linter config — convention only)
- **String unions over enums** — `type ToolVerbosity = "all" | "summary" | ...`
- **`verbatimModuleSyntax`** — use `import type` for type-only imports
- **Error surfacing** — `toFriendlyError()` for Telegram users, `console.error` for internals
- **Commits** — Conventional Commits with optional scopes

→ [Full conventions](docs/context/conventions.md)

## Build & Deploy

```bash
npm install          # install deps
npm run dev          # dev mode (tsx)
npm run build        # compile to dist/
npm test             # run tests
npm run test:coverage # tests + coverage
docker compose up    # containerized
telepi setup         # service install (macOS/Linux)
```

- macOS: launchd (`~/Library/LaunchAgents/com.telepi.plist`)
- Linux: systemd (`~/.config/systemd/user/telepi.service`)
- npm publishing: GitHub Actions + Trusted Publishing on `v*.*.*` tags

→ [Full deploy guide](docs/context/deploy.md)

## Gotchas

- **Bash timeout monkey-patch** — TelePi patches Pi SDK's bash tool to enforce 120s timeout. Fragile if SDK changes internals.
- **Bootstrap session consumed once** — First `getOrCreate()` call gets the `PI_SESSION_PATH` session; subsequent chats always create fresh.
- **Per-chat isolation = more memory** — Each chat/topic gets its own `AgentSessionRuntime`.
- **409 Conflict retry** — Long-polling restarts up to 5 times on HTTP 409 (another instance running).
- **Voice fallback chain** — parakeet-coreml → sherpa-onnx → OpenAI Whisper. Requires optional deps installed.
- **No linter** — Style enforced by convention and review only.
- **`src/install.ts` excluded from coverage** — Orchestration facade, not testable in isolation.
