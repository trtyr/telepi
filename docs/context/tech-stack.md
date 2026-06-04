# Tech Stack

## Language & Runtime

| Property | Value |
|----------|-------|
| Language | Rust |
| Edition | 2024 |
| Minimum Rust Version (MSRV) | 1.85 |
| Crate Type | Binary (`telepi`) |
| Entry Point | `src/main.rs` |

## Framework & Async Runtime

| Crate | Version Spec | Locked Version | Features |
|-------|-------------|----------------|----------|
| tokio | 1 | 1.52.3 | `full` |
| teloxide | 0.13 | 0.13.0 | `macros` |

## Dependencies

### Telegram / HTTP

| Crate | Version Spec | Locked Version | Features | Purpose |
|-------|-------------|----------------|----------|---------|
| teloxide | 0.13 | 0.13.0 | `macros` | Telegram bot framework |
| reqwest | 0.12 | 0.12.28 | `json`, `multipart`, `stream` | HTTP client (Whisper API, etc.) |
| tokio-stream | 0.1 | — | — | Async stream utilities |

### Serialization

| Crate | Version Spec | Locked Version | Features | Purpose |
|-------|-------------|----------------|----------|---------|
| serde | 1 | 1.0.228 | `derive` | Serialization/deserialization |
| serde_json | 1 | — | — | JSON support |

### CLI

| Crate | Version Spec | Locked Version | Features | Purpose |
|-------|-------------|----------------|----------|---------|
| clap | 4 | 4.6.1 | `derive` | CLI argument parsing |

### Error Handling

| Crate | Version Spec | Locked Version | Purpose |
|-------|-------------|----------------|---------|
| thiserror | 2 | 2.0.18 | Custom error type derive |
| anyhow | 1 | 1.0.102 | Ad-hoc error context |

### Logging

| Crate | Version Spec | Locked Version | Features | Purpose |
|-------|-------------|----------------|----------|---------|
| tracing | 0.1 | 0.1.44 | — | Structured logging macros |
| tracing-subscriber | 0.3 | — | `env-filter` | Log output and filtering |
| tracing-appender | 0.2 | — | — | File log output |

### Configuration

| Crate | Version Spec | Locked Version | Purpose |
|-------|-------------|----------------|---------|
| dotenvy | 0.15 | 0.15.7 | `.env` file loading |

### Utilities

| Crate | Version Spec | Locked Version | Features | Purpose |
|-------|-------------|----------------|----------|---------|
| uuid | 1 | 1.23.2 | `v4` | Session ID generation |
| chrono | 0.4 | 0.4.44 | `serde` | Timestamps and date formatting |
| regex | 1 | 1.12.3 | — | Pattern matching |
| glob | 0.3 | — | — | File glob patterns |
| sha2 | 0.10 | — | — | SHA-256 hashing |
| base64 | 0.22 | — | — | Base64 encoding/decoding |
| which | 7 | — | — | Executable lookup (PATH search) |
| dirs | 6 | — | — | Platform-standard directories |
| libc | 0.2 | — | — | Low-level C bindings |
| async-trait | 0.1 | — | — | Async trait support |

## Dev Dependencies

| Crate | Version Spec | Locked Version | Purpose |
|-------|-------------|----------------|---------|
| tokio-test | 0.4 | 0.4.5 | Async test utilities |

## Notable Transitive Dependencies

| Crate | Pulled By | Locked Version | Notes |
|-------|-----------|----------------|-------|
| reqwest | teloxide 0.13 | 0.11.27 | teloxide uses an older reqwest internally; the project also depends on reqwest 0.12 directly. Both versions coexist in the lockfile. |
| thiserror | teloxide (transitive) | 1.0.69 | Older thiserror 1.x via teloxide; project uses thiserror 2.x directly. Both coexist. |

## Build Tools

| Tool | Purpose | Config File |
|------|---------|-------------|
| cargo | Build, test, run, package | `Cargo.toml`, `Cargo.lock` |

No Makefile, justfile, `.cargo/config.toml`, `rustfmt.toml`, `clippy.toml`, or CI configuration files are present. Formatting and linting rely on default `cargo fmt` and `cargo clippy` behavior.

## Runtime Requirements

| Requirement | Details |
|-------------|---------|
| Rust toolchain | ≥ 1.85 (edition 2024) |
| Telegram Bot Token | `TELEGRAM_BOT_TOKEN` (required) |
| Allowed User IDs | `TELEGRAM_ALLOWED_USER_IDS` (required, comma-separated) |
| `pi` CLI | Must be on PATH — the bot spawns `pi` as a subprocess |

### Optional Runtime Environment

| Variable | Purpose |
|----------|---------|
| `TELEPI_WORKSPACE` | Root directory for projects |
| `TOOL_VERBOSITY` | Controls output verbosity (e.g. `summary`) |
| `TELEPI_PROMPT_INBOX_DIR` | Directory for file-based prompt inbox |
| `TELEPI_PROMPT_INBOX_INTERVAL_MS` | Polling interval for inbox (default: 60000) |
| `OPENAI_API_KEY` | Enables OpenAI Whisper voice transcription backend |
| `SHERPA_ONNX_MODEL_DIR` | Enables Sherpa-ONNX local transcription backend |
| `SHERPA_ONNX_NUM_THREADS` | Thread count for Sherpa-ONNX (default: 2) |
| `PI_SESSION_PATH` | Bootstrap session path (consumed on first use, then ignored) |
| `PI_MODEL` | Override default AI model |

## Version Constraints Summary

| Constraint | Value | Source |
|------------|-------|--------|
| MSRV | 1.85 | `Cargo.toml` (`rust-version`) |
| Rust edition | 2024 | `Cargo.toml` (`edition`) |
| teloxide | ^0.13 | `Cargo.toml` |
| tokio | ^1 | `Cargo.toml` |
| reqwest | ^0.12 | `Cargo.toml` |

## Commands

```bash
cargo build          # Debug build
cargo build --release # Release build
cargo test           # Run unit tests
cargo run -- start   # Start the bot
cargo run -- status  # Show version and config
cargo fmt            # Format code (defaults)
cargo clippy         # Lint (defaults)
```
