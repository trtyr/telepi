# Conventions

## Error Handling

Single top-level error enum `TelePiError` in `src/error.rs` using `thiserror`.

- Variants map 1:1 to domain areas: `MissingEnv`, `InvalidConfig`, `Telegram`, `PiSession`, `PiProcess`, `Voice`, `Install`, `Io`, `Http`, `Serde`, `Other`.
- `#[from]` auto-conversions for `std::io::Error`, `reqwest::Error`, `serde_json::Error`, `anyhow::Error`.
- Manual `From<teloxide::RequestError>` impl (format-as-string) because teloxide's error doesn't implement `std::error::Error` in a way that satisfies `thiserror`'s `#[from]`.
- Custom `Result<T>` alias: `pub type Result<T> = std::result::Result<T, TelePiError>`.
- `to_friendly_error()` maps each variant to a user-facing string (strips internal prefixes). Used in bot handlers before sending messages.
- **Inconsistency**: `anyhow::Result` used in `bot/mod.rs:run()` entrypoint; `teloxide::ResponseResult<()>` in command dispatch; typed `crate::error::Result` everywhere else.

## Config

Resolution order (`src/config.rs:load_config()`):
1. `TELEPI_CONFIG` env var → explicit path
2. `.env` in cwd
3. `~/.config/telepi/.env` (default)

Two internal helpers (not public):
- `require_env(name)` → `Result<String>`, maps missing to `MissingEnv`
- `optional_string(name)` → `Option<String>`, trims whitespace, filters empty

`TelePiConfig` is a flat struct (no nesting). All fields public. Loaded once, wrapped in `Arc<TelePiConfig>`, threaded through `HandlerState` and `SessionRegistry`.

## Module Organization

```
src/
├── main.rs          # Entrypoint — delegates to cli.rs
├── lib.rs           # Re-exports all top-level modules
├── cli.rs           # clap derive (Commands enum)
├── config.rs        # Env loading, TelePiConfig struct
├── error.rs         # TelePiError, Result alias, to_friendly_error()
├── format.rs        # escape_html() — single utility function
├── paths.rs         # Path constants + home_dir/expand_home helpers
├── bot/
│   ├── mod.rs       # run() — builds teloxide Dispatcher
│   ├── commands/    # BotCommands derive + dispatch + per-command modules
│   ├── handler.rs   # text/voice/photo/abort/retry handlers
│   ├── keyboard.rs  # InlineKeyboard builders
│   ├── state.rs     # BotChatState (busy guard)
│   └── transport.rs # Telegram message sending utilities
├── pi/
│   ├── mod.rs       # Re-exports
│   ├── session.rs   # PiSession trait + event types
│   ├── cli_session.rs  # CLI subprocess implementation
│   └── registry.rs  # SessionRegistry (HashMap + RwLock)
├── install/
│   └── mod.rs       # launchd/systemd service install
└── voice/
    └── mod.rs       # Transcription backends (Whisper, Parakeet, Sherpa)
```

Convention: each `mod.rs` is minimal (re-exports only, or contains the full implementation for smaller modules like `install/` and `voice/`).

## Naming & Style

- **Standard rustfmt defaults** — no `rustfmt.toml` or `clippy.toml` overrides.
- **4-space indentation**, standard Rust snake_case/PascalCase.
- **Serde**: enums use `#[serde(rename_all = "kebab-case")]` (see `ToolVerbosity`).
- **Commands**: `#[command(rename_rule = "lowercase")]` on teloxide `BotCommands` derive.
- **Struct derives**: `#[derive(Debug, Clone)]` is the baseline. `PartialEq, Eq, Hash` added for types used as map keys (`SessionContext`). Manual `Debug` impl only when inner type doesn't implement it (`SessionRegistry` wraps `Arc<RwLock<...>>`).
- **Display impls**: manual for domain types (`SessionContext`, `ToolVerbosity`), not blanket `#[derive(Display)]`.
- **Async traits**: `#[async_trait::async_trait]` on `PiSession` trait (Rust edition 2024 native async-in-traits not used yet).
- **Import order**: `std` → external crates → `crate::` imports → `self::` (relative) imports. No blank-line separator enforced between groups.

## Shared State Patterns

- `Arc<TelePiConfig>` — immutable config, shared across all handlers.
- `Arc<RwLock<HashMap<...>>>` for `SessionRegistry` (read-heavy, write-rare).
- `Arc<Mutex<...>>` for `BotChatState` (busy guard, per-chat concurrent access).
- Double-check locking in `SessionRegistry::get_or_create()` — read lock first, then upgrade to write lock.

## Anti-Patterns & Incomplete Features

6 `TODO` comments across the codebase (no FIXME/HACK/XXX/DEPRECATED found):

| Location | TODO |
|----------|------|
| `src/main.rs:35` | Interactive setup |
| `src/pi/cli_session.rs:410` | Parse session JSONL for actual stats |
| `src/pi/cli_session.rs:481` | Persist model selection |
| `src/voice/mod.rs:75` | Implement Parakeet CoreML transcription |
| `src/voice/mod.rs:81` | Implement Sherpa-ONNX transcription |
| `src/install/mod.rs:30` | Check if service is actually running |

## Other Noteworthy Patterns

- **Constants**: module-level `pub const` in dedicated files (`paths.rs`).
- **Path resolution**: `expand_home()` handles `~` prefix; `resolve_from_cwd()` handles relative paths. Used throughout config and install modules.
- **Tests**: only in `src/config.rs` (4 unit tests). No integration tests, no `tests/` directory.
- **`lib.rs`**: exposes all modules as `pub mod` — the crate is usable as a library in theory, but no external consumers.
- **Dead code**: `src/bot/commands/basic.rs` defines `cmd_start` and `cmd_help`, but `commands/mod.rs:dispatch()` uses an inline `send_welcome()` instead. The functions in `basic.rs` are never called.
