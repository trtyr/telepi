# TelePi — Build, Test & Deploy Reference

## Prerequisites

- Node.js >= 20 (Node 22 recommended)
- npm
- Docker (optional, for containerized deployment)
- macOS with launchd, or Linux with systemd (for service installation)

## Environment Variables

Copy `.env.example` and fill in the required values:

```bash
cp .env.example .env
```

| Variable | Required | Description |
|---|---|---|
| `TELEGRAM_BOT_TOKEN` | Yes | Telegram bot token from @BotFather |
| `TELEGRAM_ALLOWED_USER_IDS` | Yes | Comma-separated Telegram user IDs allowed to use the bot |
| `TELEPI_WORKSPACE` | No | Absolute path to default project workspace for new sessions |
| `TOOL_VERBOSITY` | No | `all` / `summary` / `errors-only` / `none` (default: `summary`) |
| `TELEPI_PROMPT_INBOX_DIR` | No | Directory to poll for `.txt` prompt files |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | No | Polling interval in ms (default: `60000`) |
| `OPENAI_API_KEY` | No | Enables cloud voice transcription via OpenAI Whisper (~$0.006/min) |
| `SHERPA_ONNX_MODEL_DIR` | No | Path to Sherpa-ONNX model for local offline transcription (Intel Macs) |
| `SHERPA_ONNX_NUM_THREADS` | No | Thread count for Sherpa-ONNX (default: `2`) |
| `PI_SESSION_PATH` | No | Open a specific Pi session file (usually injected by `/handoff`) |
| `PI_MODEL` | No | Override model, e.g. `anthropic/claude-sonnet-4-5` |

## Local Development

```bash
# Install dependencies
npm install

# Run in dev mode (auto-reload with tsx)
npm run dev

# Run built version
npm run build
npm start
```

## Build

```bash
# Standard build (TypeScript → dist/)
npm run build

# Clean build
npm run build:clean    # equivalent to: rm -rf dist artifacts && tsc

# Clean only
npm run clean
```

The compiled output goes to `dist/`. The CLI entrypoint is `dist/cli.js`.

## Testing

```bash
# Run tests once
npm test

# Run tests with coverage report (enforced: 85% lines/functions/statements, 75% branches)
npm run test:coverage
```

Tests use Vitest with `globals: true`. Test files live in `test/**/*.test.ts`.

## Docker

### Build and run

```bash
# Build image and start
docker compose up --build

# Run in background
docker compose up -d --build

# View logs
docker compose logs -f

# Stop
docker compose down
```

### What `docker-compose.yml` does

- Loads env from `.env` via `env_file`
- Mounts `~/.pi/agent/auth.json` and `settings.json` **read-only**
- Mounts `~/.pi/agent/sessions` **read-write** (session persistence)
- Mounts `./workspace` as `/workspace` **read-write** (agent workspace)
- Drops all Linux capabilities, enforces no-new-privileges
- Limits: 2 GB memory, 2 CPUs
- Restarts unless manually stopped

### The Dockerfile

- Base: `node:22-alpine`
- Installs git and bash
- Runs `npm ci` (or `npm install` if no lockfile), then `npm run build`
- Creates non-root `telepi` user (UID 1001)
- Sets up writable npm global prefix so the agent can install extensions at runtime
- Entrypoint: `node dist/index.js`

## macOS Service (launchd)

### Automatic setup via CLI

```bash
# Build first
npm run build

# Run interactive setup — creates ~/.config/telepi/config.env, installs plist, loads service
telepi setup

# Check service status
telepi status
```

### Manual plist installation

```bash
# Edit the template plist, replacing placeholder paths
cp launchd/com.telepi.plist ~/Library/LaunchAgents/com.telepi.plist
# Edit ~/Library/LaunchAgents/com.telepi.plist with absolute paths

# Load the agent
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.telepi.plist

# Restart the service
launchctl kickstart -k gui/$UID/com.telepi

# Check status
launchctl print gui/$UID/com.telepi
```

### Logs

Installed-mode logs are at:

```
~/Library/Logs/TelePi/telepi.out.log
~/Library/Logs/TelePi/telepi.err.log
```

### Unload

```bash
launchctl bootout gui/$UID ~/Library/LaunchAgents/com.telepi.plist
```

## Linux Service (systemd)

### Automatic setup via CLI

```bash
npm run build
telepi setup
telepi status
```

### Manual unit installation

```bash
# Copy and configure the unit template
mkdir -p ~/.config/systemd/user
cp systemd/telepi.service ~/.config/systemd/user/telepi.service
# Edit the unit file, replacing __TELEPI_*__ placeholders with actual paths

# Reload, enable, and start
systemctl --user daemon-reload
systemctl --user enable telepi.service
systemctl --user start telepi.service

# Check status
systemctl --user status telepi.service
```

### Logs

```bash
# Follow logs
journalctl --user -u telepi.service -f

# Or check the append log files configured in the unit file
cat ~/.local/state/telepi/logs/telepi.out.log
```

### Restart / stop

```bash
systemctl --user restart telepi.service
systemctl --user stop telepi.service
```

### Ensure service starts on boot (no login required)

```bash
loginctl enable-linger $USER
```

## npm Publishing

TelePi uses npm Trusted Publishing from GitHub Actions — no `NPM_TOKEN` secret needed.

### Release flow

```bash
# 1. Bump version in package.json
npm version patch   # or minor / major

# 2. Push with tag — triggers .github/workflows/release.yml
git push origin main --follow-tags
```

### What CI does on tag push

1. Checks out the repo, sets up Node 22.14
2. Verifies the git tag matches `package.json` version
3. Runs `npm ci`, then `npm test && npm run package:release`
4. Publishes to npm with `--access public --provenance`
   - Tags matching `v*.*.*-` (prerelease) get the `next` dist-tag
   - Stable tags get the default `latest` dist-tag
5. Creates a GitHub Release with `.tar.gz` and `.sha256` artifacts

### Manual pre-release

```bash
npm version prerelease --preid=beta
git push origin main --follow-tags
# CI publishes with --tag next
```

## CI Pipeline (all pushes and PRs)

`.github/workflows/ci.yml` runs on every push and pull request:

```bash
npm ci
npm run build
npm run test:coverage
```

Concurrency is grouped per PR/ref so redundant runs are cancelled.

## Installed Mode CLI Reference

| Command | Description |
|---|---|
| `telepi setup` | Interactive setup: config, service install, extension symlink |
| `telepi start` | Start the bot (used by the service unit) |
| `telepi status` | Show version, config path, service state, extension state |
