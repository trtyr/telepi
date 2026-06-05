# TelePi

> Telegram bridge for the Pi coding agent — Rust implementation

## Quick Reference

| What | Value |
|------|-------|
| Language | Rust (edition 2024) |
| Framework | teloxide 0.13 |
| Runtime | tokio (full features) |
| Package Manager | cargo |
| Entry Point | `src/main.rs` |
| Test Command | `cargo test` |
| Dev Command | `cargo run` |

## Overview

TelePi is a Rust binary that bridges Telegram messages to the Pi coding agent CLI. Users send messages to a Telegram bot, which spawns `pi` CLI subprocesses and returns formatted responses. Supports per-chat sessions, service installation (launchd/systemd), voice transcription backends, Docker deployment, and a filesystem-based prompt inbox for external tool integration.

## Architecture Overview

Three-layer design: CLI entry (`main.rs`, `cli.rs`) → Bot layer (`bot/`) → Session layer (`pi/`). Bot layer handles Telegram commands and messages, manages per-chat busy state, streams real-time progress to Telegram, and splits long messages. Session layer abstracts Pi agent interactions via `PiSession` trait (currently backed by CLI subprocess with JSON streaming protocol). Support modules provide TOML + env config loading, platform-aware path resolution, error handling, and platform-specific service installation. Prompt inbox enables external tools to inject prompts via filesystem polling.

→ [Full architecture analysis](docs/context/architecture.md)

## Key Modules

| Module | Purpose | Details |
|--------|---------|---------|
| `bot/` | Telegram bot handler, commands, state, transport, prompt inbox | [→ modules.md](docs/context/modules.md) |
| `pi/` | Pi agent session management (trait + CLI impl + registry + tree) | [→ modules.md](docs/context/modules.md) |
| `install/` | Service installation for launchd/systemd | [→ modules.md](docs/context/modules.md) |
| `voice/` | Voice message transcription (3 backends) | [→ modules.md](docs/context/modules.md) |
| `cli.rs` | CLI argument parsing (clap) | [→ modules.md](docs/context/modules.md) |
| `config.rs` | TOML + env config loading, validation | [→ modules.md](docs/context/modules.md) |

## Tech Stack

| Category | Choice | Version |
|----------|--------|---------|
| Language | Rust | edition 2024, rust-version 1.85 |
| Async Runtime | tokio | 1 (full features) |
| Telegram Bot | teloxide | 0.13 (macros) |
| HTTP Client | reqwest | 0.11 (json, multipart, stream, socks) |
| CLI | clap | 4 (derive) |
| Error Handling | thiserror + anyhow | 2 + 1 |
| Logging | tracing + tracing-subscriber | 0.1 + 0.3 |
| Config | dotenvy + toml | 0.15 + 0.8 |

→ [Full dependency analysis](docs/context/tech-stack.md)

## Commands

```bash
# Build
cargo build --release

# Run
cargo run -- start    # Start the bot (default)
cargo run -- setup    # Interactive setup (--bot-token, --user-ids, --workspace)
cargo run -- status   # Show installation status

# Test
cargo test

# Docker
docker build -t telepi .
docker compose up -d

# Environment
cp telepi.toml ~/.config/telepi/config.toml  # TOML config (primary)
cp .env.example .env  # Or use .env as fallback
```

## Conventions

- **Config resolution**: `TELEPI_CONFIG` env → `./telepi.toml` → `~/.config/telepi/config.toml`; env vars override individual TOML fields
- **Error handling**: `thiserror` enum (`TelePiError`) with `to_friendly_error()` for user-facing messages
- **Session management**: per-chat isolation via `SessionRegistry` (HashMap + RwLock)
- **Busy guard**: prevents concurrent prompts per chat with `BotChatState` (Arc<Mutex>)
- **Streaming**: `PiEvent` enum + mpsc channel for real-time Telegram progress updates
- **Prompt inbox**: filesystem polling for `.txt` files, configurable interval

→ [Full conventions](docs/context/conventions.md)

## Public Interfaces

### CLI Commands

| Command | Description |
|---------|-------------|
| `telepi start` | Start the Telegram bot (default) |
| `telepi setup` | Interactive setup (`--bot-token`, `--user-ids`, `--workspace`) |
| `telepi status` | Show version, config, service status |

### Telegram Bot Commands

| Command | Description |
|---------|-------------|
| `/start`, `/help` | Welcome message and command list |
| `/new` | Create a fresh session |
| `/sessions` | List and switch sessions |
| `/handback` | Resume session in terminal |
| `/abort` | Cancel running operation |
| `/retry` | Re-send last prompt |
| `/model` | Show/set current AI model (inline keyboard picker) |
| `/context` | Show session stats |
| `/tree` | View conversation tree |

→ [Full API reference](docs/context/api.md)

## Gotchas

- **Dead code**: `commands/tree.rs` defines `cmd_branch` and `cmd_label` stubs not wired into the `Command` enum
- **Bootstrap path**: `PI_SESSION_PATH` is consumed by first session creation, then ignored
- **Unimplemented features**: voice Parakeet/Sherpa-ONNX backends are stubs, stats returns zeros, set_model is no-op
- **Docker**: Dockerfile and docker-compose.yml present for container deployment
- **14 unit tests**: across `config.rs` (7), `format.rs` (5), `pi/tree.rs` (2) — no integration tests
- **409 Conflict retry**: bot retries Telegram 409 errors up to 5× with 3s delay (handles competing instances)

---

*Generated by /init-local on 2026-06-05.*
