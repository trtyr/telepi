# Modules

| Module | Path | Responsibility |
|---|---|---|
| bot | `src/bot/` | Telegram bot handler, commands, state, transport, prompt inbox |
| pi | `src/pi/` | Pi agent session management (trait + CLI impl + registry + tree) |
| install | `src/install/` | Service installation (launchd/systemd), status detection |
| voice | `src/voice/` | Voice transcription (3 backends, only OpenAI wired) |
| cli | `src/cli.rs` | CLI argument parsing (clap) |
| config | `src/config.rs` | TOML + env config loading, validation |
| error | `src/error.rs` | Error types (thiserror), friendly display |
| format | `src/format.rs` | HTML escaping for Telegram |
| paths | `src/paths.rs` | Platform-aware path resolution |
| main | `src/main.rs` | Entrypoint, tokio runtime, proxy setup, process cleanup |

---

## `bot/` — Telegram Bot Layer

**Key files:** `mod.rs` (105), `handler.rs` (490), `state.rs` (107), `transport.rs` (131), `prompt_inbox.rs` (108), `keyboard.rs` (71), `commands/` (5 files)

**Public surface:** `run(config)` → builds teloxide Dispatcher with dptree filter chain (command → voice → photo → text). `HandlerState { config, sessions, chat_state, model_lists }` shared via `dptree::deps!`. Commands: `/new`, `/sessions`, `/handback`, `/abort`, `/retry`, `/model` (inline keyboard picker + `handle_model_callback`), `/tree` (renders session tree via `pi::tree`), `/context`.

**Internal wiring:**
- `handler.rs` → `state`, `transport`, `config`, `format`, `pi::registry`, `pi::session::PiEvent`, `pi::cli_session::ModelInfo`
- `prompt_inbox.rs` → `handler::HandlerState`, `state`, `config`
- `commands/model.rs` → `pi::cli_session::CliSession::list_models`
- `commands/tree.rs` → `pi::tree` (JSONL parsing + rendering)
- `transport.rs`, `keyboard.rs` → teloxide only (no internal deps)

**Notable patterns:**
- **Busy guard**: every handler checks `state.is_busy(key)` before processing
- **Streaming prompt**: `process_prompt` uses `prompt_streaming` + `mpsc::channel<PiEvent>`; spawned task applies debounced edits (1.5s) to the Telegram message
- **Retry**: `transport::with_retry` retries network errors up to 3× with exponential backoff (2s × attempt); `mod.rs` retries 409 Conflict up to 5× with 3s delay
- **Prompt inbox**: background task polls `prompt_inbox_dir` for `.txt` files at `prompt_inbox_interval_ms` (default 60s), processes oldest-first, uses synthetic `"inbox"` chat key
- **`basic.rs` deleted**: `cmd_start`/`cmd_help` replaced by inline `send_welcome()` in `commands/mod.rs`
- **Dead code**: `commands/tree.rs` defines `cmd_branch` and `cmd_label` (both stubs, not wired into `Command` enum)

---

## `pi/` — Pi Agent Session Layer

**Key files:** `session.rs` (118), `registry.rs` (90), `cli_session.rs` (548), `tree.rs` (350)

**Public surface:**
- `PiSession` trait: `prompt`, `prompt_with_images`, `prompt_streaming`, `abort`, `set_model`, `dispose`, `info`, `stats`
- `SessionRegistry`: `get_or_create(ctx)`, `remove(ctx)`, `list()` — HashMap + `Arc<RwLock>`
- `CliSession`: `create(config, ctx, bootstrap_path)`, `pi_cli_available()`, `list_models()`
- `pi::tree`: `parse_session_jsonl`, `build_tree`, `render_tree`, `find_session_dirs`, `find_latest_session_file` — reads Pi's native `~/.pi/agent/sessions/` JSONL format

**Data types:** `SessionContext { chat_id, message_thread_id }`, `SessionInfo`, `PromptResponse { text, tool_calls }`, `PiEvent` (9 variants: ThinkingDelta, TextDelta, ToolStart/Output/End, Usage, TurnEnd, Error), `ModelInfo { provider, model, context_window, ... }`, `SessionEntry`/`TreeNode` (tree module)

**Key details:**
- **Bootstrap path**: `PI_SESSION_PATH` is consumed by first `get_or_create` (`Option::take`), subsequent chats get fresh sessions
- **Streaming JSON protocol**: spawns `pi --mode json --print <text>`, reads stdout line-by-line as `JsonEvent` (tagged enum), translates to `PiEvent`s
- **Abort**: stores `running_child: Arc<Mutex<Option<Child>>>`, sends `SIGTERM` (unix only)
- **Not yet implemented**: `stats()` returns zeros, `set_model()` is no-op, `prompt_with_images` uses `@file` CLI syntax
- **Tree module** is new: reads `session.jsonl` files from `~/.pi/agent/sessions/<encoded-workspace>/<uuid>/run-N/`, builds parent-child tree, renders with box-drawing chars (max_depth=4, max_entries=30)

---

## `install/` — Service Installation

**Key files:** `mod.rs` (74), `platform.rs` (53), `launchd.rs` (62), `systemd.rs` (43)

**Public surface:** `get_status() → TelePiStatus { version, config_path, service, extension }`, `detect_platform()`, `build_plist()`/`build_unit()` for generating service files.

**Key details:** Extension detection looks for `~/.pi/agent/extensions/telepi-handoff.ts`. Service running check is TODO. Used by `Commands::Status`.

---

## `voice/` — Voice Transcription

**Key file:** `mod.rs` (~131 lines)

**Public surface:** `transcribe(file_path) → TranscriptionResult { text, backend, duration_ms }`, `available_backends()`, `VoiceBackend` enum (Parakeet, SherpaOnnx, OpenAi).

**Key details:** Priority: Parakeet (macOS ARM) > Sherpa-ONNX > OpenAI Whisper. Only OpenAI is implemented. Wired into `voice_handler` in `bot/handler.rs`.

---

## `cli.rs` — CLI Definition

`Cli` struct with optional `Commands` subcommand: `Start` (default, starts bot), `Setup` (prints `telepi.toml` template), `Status` (calls `install::get_status()`).

---

## `config.rs` — Configuration

**Config resolution:** `TELEPI_CONFIG` env → `./telepi.toml` → `~/.config/telepi/config.toml`. TOML values can be overridden by env vars.

**TOML structure:** `[telegram]` (bot_token, allowed_user_ids), `[pi]` (workspace, model, session_path, tool_verbosity), `[prompt_inbox]` (dir, interval_ms), `[voice]` (openai_api_key, sherpa_onnx_model_dir, sherpa_onnx_num_threads), top-level `proxy`, `log_level`.

**Key types:** `TomlConfig` (serde), `TelePiConfig` (resolved), `ConfigSource` (Toml|EnvOnly|Missing), `ToolVerbosity` (All|Summary|ErrorsOnly|None).

**Workspace resolution:** `TELEPI_WORKSPACE` → `pi.workspace` → `/workspace` (Docker) → cwd → `.`
**Proxy resolution:** `HTTP_PROXY` → `HTTPS_PROXY` → `ALL_PROXY` → `toml.proxy`
**Log level:** `RUST_LOG` → `toml.log_level` → `"info"`

---

## `error.rs` — Error Handling

`TelePiError` enum (thiserror): `MissingEnv`, `InvalidConfig`, `Telegram`, `PiSession`, `PiProcess`, `Voice`, `Install`, `Io`, `Http`, `Serde`, `Other`. All string-wrapped except the `From` impls. `teloxide::RequestError` → `Telegram(String)`. Helper: `to_friendly_error()`.

---

## `paths.rs` + `format.rs` — Utilities

**paths:** `home_dir()`, `expand_home()`, `resolve_from_cwd()`, `default_config_dir/path()`, `default_systemd_user_dir()`, `default_log_dir()`, `DOCKER_WORKSPACE_PATH` (`"/workspace"`).

**format:** `escape_html(text)` — replaces `&`, `<`, `>` for Telegram HTML parse mode.

---

## `main.rs` — Entrypoint

**Startup flow:** parse CLI → match command → for `Start`: `load_config()`, `kill_existing_processes()` (pgrep/pkill), set `HTTP_PROXY`/`HTTPS_PROXY` from config, build tokio multi-thread runtime, init tracing with `cfg.log_level`, call `bot::run(cfg)`.

**`kill_existing_processes()`:** kills stale `telepi` and `pi --mode json` processes via `pgrep -f`/`pkill -f` before starting.
