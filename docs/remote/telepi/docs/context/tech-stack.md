# TelePi Tech Stack

## Language & Runtime

| Property | Value |
|---|---|
| Language | TypeScript ^5.7.0 |
| Runtime | Node.js >= 20 |
| Module system | ESM (`"type": "module"`) |
| Package name | `@futurelab-studio/telepi` v0.4.2 |
| License | MIT |

## TypeScript Configuration

Source: `tsconfig.json`

| Option | Value |
|---|---|
| Target | ES2022 |
| Module / Resolution | Node16 / Node16 |
| Strict mode | true |
| `verbatimModuleSyntax` | true |
| `esModuleInterop` | true |
| `skipLibCheck` | true |
| `rootDir` / `outDir` | `src` / `dist` |
| `forceConsistentCasingInFileNames` | true |
| `types` | `["node"]` |

## Production Dependencies

### Telegram

| Package | Version | Purpose |
|---|---|---|
| `grammy` | ^1.35.0 | Telegram Bot API framework |
| `@grammyjs/auto-retry` | ^2.0.2 | Automatic retry for Telegram API calls |

### Pi Agent

| Package | Version | Purpose |
|---|---|---|
| `@mariozechner/pi-coding-agent` | 0.70.2 (pinned) | Pi coding agent SDK |

### Utilities

| Package | Version | Purpose |
|---|---|---|
| `minimatch` | ^10.2.4 | Glob pattern matching for file filtering |

## Optional Dependencies (Voice Transcription)

| Package | Version | Purpose | Platform |
|---|---|---|---|
| `parakeet-coreml` | ^2.2.0 | CoreML-based speech transcription | Apple Silicon |
| `sherpa-onnx-node` | 1.12.32 (pinned) | ONNX-based speech transcription | Cross-platform |

Voice transcription falls back through three tiers: local CoreML → local Sherpa-ONNX → cloud OpenAI Whisper. Tiers are selected by which optional packages are installed and which environment variables are set.

## Dev Dependencies

| Package | Version | Purpose |
|---|---|---|
| `typescript` | ^5.7.0 | TypeScript compiler |
| `@types/node` | ^22.0.0 | Node.js type definitions |
| `tsx` | ^4.19.0 | TypeScript execution for development |
| `vitest` | ^3.2.4 | Test framework |
| `@vitest/coverage-v8` | ^3.2.4 | V8-based code coverage |

## Build & Run Commands

| Command | Tool | Description |
|---|---|---|
| `npm run build` | `tsc` | Compile TypeScript to `dist/` |
| `npm run dev` | `tsx src/index.ts` | Development mode with live TS execution |
| `npm start` | `node dist/index.js` | Run compiled build |
| `npm run clean` | `rm -rf dist artifacts` | Remove build output and packaging artifacts |
| `npm run build:clean` | `npm run clean && npm run build` | Clean build from scratch |
| `npm run package:release` | `npm run build:clean && node scripts/package-release.mjs` | Build + produce distributable archive |
| `npm run ci:release` | `npm test && npm run package:release` | Full CI gate: test then package |
| `npm test` | `vitest run` | Run test suite once |
| `npm run test:coverage` | `vitest run --coverage` | Tests with V8 coverage |

## Test Framework

Source: `vitest.config.ts`

| Setting | Value |
|---|---|
| Framework | Vitest ^3.2.4 |
| Globals | true |
| Test pattern | `test/**/*.test.ts` |
| Coverage provider | V8 |
| Coverage includes | `src/**/*.ts` |
| Coverage excludes | `src/index.ts`, `src/install.ts` |

### Coverage Thresholds

| Metric | Minimum |
|---|---|
| Lines / Functions / Statements | 85% |
| Branches | 75% |

## Container Runtime

Source: `Dockerfile`, `docker-compose.yml`

| Property | Value |
|---|---|
| Base image | `node:22-alpine` |
| System packages | `git`, `bash` |
| Process user | `telepi` (uid 1001, non-root) |
| npm global prefix | `/home/telepi/.npm-global` |
| Entry command | `node dist/index.js` |
| Memory / CPU limits | 2 GB / 2.0 cores |
| Security | `cap_drop: ALL`, `no-new-privileges` |
| Restart policy | `unless-stopped` |

### Volumes

| Mount | Mode | Purpose |
|---|---|---|
| `~/.pi/agent/auth.json` | read-only | Pi agent authentication |
| `~/.pi/agent/settings.json` | read-only | Pi agent settings |
| `~/.pi/agent/sessions` | read-write | Session files |
| `./workspace` | read-write | Agent working directory |

## Service Management

| Platform | Mechanism | Config File |
|---|---|---|
| macOS | launchd | `launchd/com.telepi.plist` |
| Linux | systemd | `systemd/telepi.service` |

Both services run `telepi start` (via `dist/cli.js`) with automatic restart on failure.

**systemd note:** `RestartSec=5` (5-second delay before restart).

## Environment Variables

Source: `.env.example`

### Required

| Variable | Description |
|---|---|
| `TELEGRAM_BOT_TOKEN` | Telegram Bot API token |
| `TELEGRAM_ALLOWED_USER_IDS` | Comma-separated Telegram user IDs |

### Optional

| Variable | Default | Description |
|---|---|---|
| `TELEPI_WORKSPACE` | — | Absolute path for new session working directory |
| `TOOL_VERBOSITY` | `summary` | Tool output level: `all` / `summary` / `errors-only` / `none` |
| `TELEPI_PROMPT_INBOX_DIR` | — | Directory to poll for `.txt` prompt files |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | `60000` | Poll interval for prompt inbox |
| `OPENAI_API_KEY` | — | Enables cloud voice transcription via OpenAI Whisper |
| `SHERPA_ONNX_MODEL_DIR` | — | Path to Sherpa-ONNX model for local transcription |
| `SHERPA_ONNX_NUM_THREADS` | `2` | Thread count for Sherpa-ONNX inference |
| `PI_SESSION_PATH` | — | Specific Pi session file for hand-off |
| `PI_MODEL` | — | Override Pi agent model |
