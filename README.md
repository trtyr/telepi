# TelePi

[![Crates.io](https://img.shields.io/crates/v/telepi?style=flat-square&logo=rust)](https://crates.io/crates/telepi)
[![Rust](https://img.shields.io/badge/rust-1.85+-ed8225?style=flat-square&logo=rust&logoColor=white)](https://rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-22C55E?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20·%20Linux-8B5CF6?style=flat-square)]()

**Telegram bridge for the Pi coding agent.** Send messages to a Telegram bot, get streamed responses from `pi` CLI. Built on teloxide + tokio. Supports per-chat sessions, voice transcription, image processing, and background service management.

[🔧 Quick Start](#quick-start) · [📡 Commands](#commands) · [⚙️ Configuration](#configuration) · [🏗️ Architecture](#architecture) · [🔧 Building](#building)

## Quick Start

```bash
# Install from crates.io
cargo install telepi

# Create config
mkdir -p ~/.config/telepi
cat > ~/.config/telepi/config.toml << 'EOF'
[telegram]
bot_token = "your-bot-token"
allowed_user_ids = [your-user-id]

[pi]
tool_verbosity = "summary"
EOF

# Run
telepi start
```

Or run as a background service:

```bash
telepi gateway start    # Install and start
telepi gateway stop     # Stop
telepi gateway restart  # Restart
```

## Commands

### CLI

| Command | Description |
|---------|-------------|
| `telepi start` | Start the Telegram bot (default) |
| `telepi gateway start` | Install and start as background service |
| `telepi gateway stop` | Stop the background service |
| `telepi gateway restart` | Restart the background service |
| `telepi status` | Show version, config, service status |
| `telepi setup` | Show config template |

### Telegram Bot

| Command | Description |
|---------|-------------|
| `/start`, `/help` | Welcome message and command list |
| `/new` | Create a fresh session |
| `/sessions` | List and switch sessions |
| `/handback` | Resume session in terminal |
| `/abort` | Cancel running operation |
| `/retry` | Re-send last prompt |
| `/model` | Show/set current AI model |
| `/context` | Show session stats |
| `/tree` | View conversation tree |

## Configuration

TelePi loads config from (in order of priority):

1. `TELEPI_CONFIG` environment variable
2. `./telepi.toml` (current directory)
3. `~/.config/telepi/config.toml`

Environment variables override individual TOML fields.

```toml
# HTTP proxy for Telegram API (http/https/socks5)
proxy = "http://127.0.0.1:7890"

# Log level: trace, debug, info, warn, error
log_level = "info"

[telegram]
bot_token = "your-bot-token"
allowed_user_ids = [123456789]

[pi]
tool_verbosity = "summary"  # all, summary, errors-only, none

[voice]
backend = "openai-whisper"  # openai-whisper, parakeet, sherpa-onnx

[prompt_inbox]
enabled = false
poll_interval_secs = 5
```

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Telegram                    │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│               Bot Layer                      │
│  commands/  handler  state  transport        │
│  prompt_inbox/  model picker  streaming      │
└──────────────────┬──────────────────────────┘
                   │  PiSession trait
┌──────────────────▼──────────────────────────┐
│              Session Layer                    │
│  CliSession (CLI subprocess + JSON stream)   │
│  SessionRegistry (per-context isolation)     │
└──────────────────┬──────────────────────────┘
                   │
┌──────────────────▼──────────────────────────┐
│            Support Modules                    │
│  config  paths  error  install  voice  format│
└─────────────────────────────────────────────┘
```

Three-layer design: **CLI entry** → **Bot layer** (Telegram handling) → **Session layer** (Pi agent abstraction). The `PiSession` trait enables swapping implementations without touching the bot layer.

## Building

- **Rust** ≥ 1.85 (edition 2024)
- **No C library required** — TelePi is pure Rust

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Docker
docker build -t telepi .
docker compose up -d
```

## Features

- **Per-chat sessions** — isolated conversation state per Telegram chat
- **Streaming responses** — real-time progress updates as Pi generates output
- **Voice transcription** — send voice messages, get transcribed and processed
- **Image processing** — send photos for visual analysis
- **Model picker** — switch AI models via inline keyboard
- **Prompt inbox** — inject prompts from filesystem (`.txt` polling)
- **Background service** — launchd (macOS) / systemd (Linux) integration
- **Conversation tree** — view full session history with box-drawing rendering

## License

MIT
