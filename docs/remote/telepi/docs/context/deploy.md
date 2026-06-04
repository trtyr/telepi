# TelePi Deploy Reference

## Build

```bash
# Install dependencies
npm install

# Compile TypeScript to dist/
npm run build

# Clean build
npm run build:clean    # rm -rf dist artifacts && tsc
```

Requirements: Node.js ≥ 20, npm.

## Run

```bash
# Development (tsx, no build step)
npm run dev

# Production (after build)
npm start              # node dist/index.js

# Or via global CLI
npm install -g @futurelab-studio/telepi
telepi start
```

## Test

```bash
# Run all tests
npm test               # vitest run

# With coverage
npm run test:coverage  # vitest run --coverage
```

Framework: Vitest ^3.2.4, globals enabled, pattern `test/**/*.test.ts`.

Coverage thresholds: 85% lines/functions/statements, 75% branches. Excludes `src/index.ts` and `src/install.ts`.

## Environment Variables

Config resolution: `TELEPI_CONFIG` env → `.env` in cwd → `~/.config/telepi/config.env`.

### Required

| Variable | Description |
|---|---|
| `TELEGRAM_BOT_TOKEN` | Telegram Bot API token from @BotFather |
| `TELEGRAM_ALLOWED_USER_IDS` | Comma-separated numeric Telegram user IDs |

### Optional

| Variable | Default | Description |
|---|---|---|
| `TELEPI_WORKSPACE` | `process.cwd()` | Working directory for new sessions |
| `TOOL_VERBOSITY` | `summary` | `all` / `summary` / `errors-only` / `none` |
| `TELEPI_PROMPT_INBOX_DIR` | — | Directory to poll for `.txt` prompt files |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | `60000` | Poll interval (ms) |
| `OPENAI_API_KEY` | — | Enables cloud voice transcription (Whisper) |
| `SHERPA_ONNX_MODEL_DIR` | — | Path to Sherpa-ONNX model for local transcription |
| `SHERPA_ONNX_NUM_THREADS` | `2` | Thread count for Sherpa-ONNX |
| `PI_SESSION_PATH` | — | Specific Pi session file for hand-off |
| `PI_MODEL` | — | Override Pi agent model |
| `TELEPI_HANDOFF_MODE` | `auto` | `auto` / `direct` / `launchd` / `systemd` |

## Service Installation

### macOS (launchd)

Installed via `telepi setup`. Plist template in `launchd/com.telepi.plist`.

```bash
# Install (done by telepi setup)
# Plist installed to: ~/Library/LaunchAgents/com.telepi.plist

# Restart after rebuild
launchctl kickstart -k gui/$UID/com.telepi

# Check status
launchctl list | grep telepi

# Logs
~/Library/Logs/TelePi/telepi.{out,err}.log
```

### Linux (systemd --user)

Unit template in `systemd/telepi.service`.

```bash
# Install (done by telepi setup)
# Unit installed to: ~/.config/systemd/user/telepi.service

systemctl --user daemon-reload
systemctl --user enable telepi
systemctl --user start telepi

# Restart
systemctl --user restart telepi

# Logs
journalctl --user -u telepi -f
# Also: ~/.local/state/telepi/logs/telepi.{out,err}.log
```

Unit: `Restart=on-failure`, `RestartSec=5`.

## Docker

```bash
# Build and run
docker compose up --build

# Or manual
docker build -t telepi .
docker run --env-file .env \
  -v ~/.pi/agent/auth.json:/home/telepi/.pi/agent/auth.json:ro \
  -v ~/.pi/agent/settings.json:/home/telepi/.pi/agent/settings.json:ro \
  -v ~/.pi/agent/sessions:/home/telepi/.pi/agent/sessions \
  -v ./workspace:/workspace \
  telepi
```

Docker specs: `node:22-alpine`, non-root user `telepi` (uid 1001), 2 GB memory limit, 2.0 CPU cores, `cap_drop: ALL`, `no-new-privileges`.

## CI / Release

CI via GitHub Actions: `.github/workflows/release.yml`. Triggers on tag push matching `v*.*.*`.

Uses npm Trusted Publishing from GitHub Actions (no `NPM_TOKEN` secret needed).

```bash
# Release flow
npm version patch|minor|major
git push origin main --follow-tags
```

Uses `npx --yes npm@11.10.0` for publish (npm ≥ 11.5.1 required for Trusted Publishing).

## Packaging

```bash
# Build + produce distributable archive
npm run package:release

# Full CI gate
npm run ci:release    # test + package
```

`scripts/package-release.mjs` creates the release artifact.

## File Locations

| What | Path |
|---|---|
| Config | `~/.config/telepi/config.env` |
| macOS service | `~/Library/LaunchAgents/com.telepi.plist` |
| Linux service | `~/.config/systemd/user/telepi.service` |
| macOS logs | `~/Library/Logs/TelePi/` |
| Linux logs | `~/.local/state/telepi/logs/` |
| Pi auth | `~/.pi/agent/auth.json` |
| Pi sessions | `~/.pi/agent/sessions/` |
| Extension | `~/.pi/agent/extensions/telepi-handoff.ts` (symlink) |
| Docker workspace | `/workspace` |
