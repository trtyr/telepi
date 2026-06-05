# Deploy Guide

## Build

```bash
cargo build --release          # Release binary
cargo build                    # Debug (faster, larger)
cargo check                    # Compile check only
ls target/release/telepi       # Binary location
```

**Toolchain:** Rust >= 1.85 (edition 2024), cargo via rustup.

## Test

```bash
cargo test                              # All tests
cargo test -- --nocapture              # With stdout
cargo test test_parse_allowed_user_ids  # Single test
cargo test config::tests               # Module
```

Framework: `#[test]` + `tokio-test` (dev-dep) for async. **14 unit tests** across 3 files:

| File | Tests | Coverage |
|------|-------|----------|
| `src/config.rs` | 7 | User ID parsing, tool verbosity, config loading |
| `src/format.rs` | 5 | Message formatting, splitting |
| `src/pi/tree.rs` | 2 | Conversation tree parsing |

No integration tests or `tests/` directory.

## Config File (`telepi.toml`)

TOML config is the primary configuration method. Resolution order:

1. `TELEPI_CONFIG` env var -> explicit path to `.toml` file
2. `./telepi.toml` in current working directory
3. `~/.config/telepi/config.toml` (default)

Env vars override individual TOML fields (see below).

```bash
cp telepi.toml ~/.config/telepi/config.toml
```

**Structure:** `[telegram]` (bot_token, allowed_user_ids), `[pi]` (workspace, model, session_path, tool_verbosity), `[prompt_inbox]` (dir, interval_ms), `[voice]` (openai_api_key, sherpa_onnx_model_dir, sherpa_onnx_num_threads). Top-level: `proxy`, `log_level`. See `src/config.rs:54-92` for full types.

## Environment Variables

Resolution: env var overrides -> TOML file -> defaults.

```bash
cp .env.example .env && vim .env
```

**Required:**

| Variable | Description |
|----------|-------------|
| `TELEGRAM_BOT_TOKEN` | Bot token from @BotFather |
| `TELEGRAM_ALLOWED_USER_IDS` | Comma-separated allowed Telegram user IDs |

**Optional:**

| Variable | Default | Description |
|----------|---------|-------------|
| `TELEPI_CONFIG` | -- | Explicit path to `.toml` config file |
| `TELEPI_WORKSPACE` | `.` (or `/workspace` in Docker) | Projects directory for Pi agent |
| `TOOL_VERBOSITY` | `summary` | `all` / `summary` / `errors-only` / `none` |
| `TELEPI_PROMPT_INBOX_DIR` | -- | Prompt inbox directory |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | `60000` | Inbox polling interval |
| `OPENAI_API_KEY` | -- | Whisper voice transcription |
| `SHERPA_ONNX_MODEL_DIR` | -- | Sherpa-ONNX local voice model path |
| `SHERPA_ONNX_NUM_THREADS` | `2` | Sherpa-ONNX thread count |
| `PI_SESSION_PATH` | -- | Bootstrap session path (consumed once) |
| `PI_MODEL` | -- | Override AI model for Pi agent |

## CI

**None configured.** No `.github/workflows/`, no Dockerfile, no Makefile.

## Service Installation

### macOS -- launchd

Generates `~/Library/LaunchAgents/com.telepi.plist` (`RunAtLoad`, `KeepAlive`). Logs at `~/Library/Logs/TelePi/`.

```bash
cp target/release/telepi /usr/local/bin/telepi
launchctl load ~/Library/LaunchAgents/com.telepi.plist
launchctl unload ~/Library/LaunchAgents/com.telepi.plist
launchctl list | grep telepi
```

### Linux -- systemd

Generates `~/.config/systemd/user/telepi.service` (`Restart=on-failure`). Logs at `~/.local/state/telepi/logs/`.

```bash
cp target/release/telepi ~/.local/bin/telepi
systemctl --user daemon-reload
systemctl --user enable --now telepi
systemctl --user status telepi
journalctl --user -u telepi -f
```

### Status

```bash
telepi status    # Version, config, service status
```

## Infrastructure Requirements

| Requirement | Details |
|-------------|---------|
| OS | macOS or Linux (Windows unsupported) |
| Rust | >= 1.85 |
| Network | Outbound HTTPS to `api.telegram.org` |
| Telegram | Bot token from @BotFather |
| Pi CLI | `pi` in `$PATH` (TelePi spawns it as subprocess) |
| Disk | Session data in `~/.pi/` (managed by Pi agent) |

## Full Deploy

```bash
cargo build --release
cargo test
cp telepi.toml ~/.config/telepi/config.toml  # Or create ./telepi.toml
# Edit config: set bot_token + allowed_user_ids
cargo run -- status                     # Verify config
cargo run -- start                      # Test foreground
# Install as service via launchd/systemd instructions above
```
