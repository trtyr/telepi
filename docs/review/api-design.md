# API Design Review

Reviewed: 2026-06-05
Scope: Public interfaces of TelePi — CLI, Telegram commands, data models, traits, config, and error types.

---

## 1. Consistency

| Score | **B** |
|-------|-------|
| Evidence | `src/pi/session.rs:82-117` — `PiSession` trait uses `&str` for text, `&[PathBuf]` for images, `Result<T>` for all fallible methods. Consistent naming: `prompt`, `prompt_with_images`, `prompt_streaming`. All methods async except `info()`. `src/config.rs:13-18` — `ToolVerbosity` enum uses `serde(rename_all = "kebab-case")` and implements `Display` + `from_str_loose`. `src/bot/state.rs:9-14` — `ChatStatus` enum uses bare PascalCase (no serde). Naming mismatch: `ToolVerbosity::ErrorsOnly` vs kebab-case serialization `"errors-only"` is handled, but the enum variant itself doesn't match the pattern. `src/error.rs:5-38` — `TelePiError` uses `String` wrappers for most variants (`PiSession(String)`, `PiProcess(String)`, `Voice(String)`), while `Io` and `Http` use `#[from]` — inconsistent wrapping pattern. |
| Recommendation | Standardize error variant inner types: either all `String` or all wrapped errors with `#[from]`, not a mix. Rename `ChatStatus` variants to use consistent naming with other enums. Consider making `PiSession::info()` async for uniformity (even if trivially sync). |

## 2. Discoverability

| Score | **A** |
|-------|-------|
| Evidence | `src/pi/session.rs:77-81` — `PiSession` trait has doc comments on every method and the trait itself. `src/config.rs:104-106` — `TelePiConfig` doc comment explains purpose. `src/error.rs:3-4` — `TelePiError` has `#[error("...")]` on every variant, self-documenting. `src/bot/state.rs:7,16,20,43` — `ChatStatus`, `ChatKey`, helper functions, and `BotChatState` all have doc comments. `docs/context/api.md` — comprehensive API reference exists. `src/bot/commands/mod.rs:12-54` — `Command` enum uses teloxide `#[command(description = "...")]` for Telegram menu registration. |
| Recommendation | Good discoverability overall. The main gap: `ModelInfo` (`src/pi/cli_session.rs:152-161`) has no doc comments on its fields. Add field-level docs for `context_window` format (e.g., "human-readable string like '200k'"). |

## 3. Backward Compatibility

| Score | **C** |
|-------|-------|
| Evidence | `src/config.rs:140` — `load_config()` is the single config entry point, returning a concrete `TelePiConfig` struct. Adding fields to `TelePiConfig` is a breaking change for any downstream consumer. `src/pi/session.rs:58-75` — `PiEvent` is an enum; adding variants is non-breaking in Rust (exhaustive match requires `_`), but consumers must handle `Error` and `TurnEnd` variants. `src/error.rs:5` — `TelePiError` is `non_exhaustive`-free; adding variants breaks match arms downstream. `src/bot/commands/mod.rs:14` — `Command` enum derived via teloxide `BotCommands`; adding commands is non-breaking. `src/pi/cli_session.rs:42-65` — `JsonEvent` uses `#[serde(other)]` for unknown events — good forward compat for pi CLI changes. |
| Recommendation | Add `#[non_exhaustive]` to `TelePiError`, `PiEvent`, and `ToolVerbosity` to allow future extension without breaking changes. Consider making `TelePiConfig` fields private with accessor methods, or at minimum `#[non_exhaustive]`. |

## 4. Input Validation

| Score | **B** |
|-------|-------|
| Evidence | `src/config.rs:144-146` — `telegram_bot_token` is required: returns `InvalidConfig` if missing. `src/config.rs:148-159` — `telegram_allowed_user_ids` validated via `parse_allowed_user_ids` which rejects non-numeric strings. `src/config.rs:27-35` — `ToolVerbosity::from_str_loose` silently falls back to `Summary` on garbage input (lenient validation). `src/config.rs:185` — `prompt_inbox_interval_ms` uses `.parse::<u64>().ok()` which silently ignores invalid values. `src/bot/handler.rs:50-55` — text handler rejects empty `None` text/from. `src/bot/handler.rs:57-62` — auth check at boundary. `src/pi/cli_session.rs:199` — `CliSession::create` does not validate that `workspace` exists. |
| Recommendation | Validate `workspace` existence at config load time (warn if missing, don't silently create). Make `ToolVerbosity::from_str_loose` return `Result` or log a warning on fallback. Validate `prompt_inbox_interval_ms` is reasonable (e.g., >1000ms). Add input length validation for prompt text to prevent unbounded sends. |

## 5. Documentation

| Score | **B+** |
|-------|-------|
| Evidence | `src/pi/session.rs:4-9,20-21,30-31,37-38,46-47,56-57,77-81,83-116` — All public types and trait methods have doc comments. `src/error.rs:3` — Error enum documented. `src/config.rs:104-106,125-126,132-139` — Config struct and loading function documented with resolution order. `src/bot/state.rs:7,16-17,20,28,43` — All public items documented. `src/voice/mod.rs:5-6,13-14,34,59` — Voice types and main function documented. `src/pi/cli_session.rs:15-18,152,193-194,232-233` — CliSession struct and methods documented. `src/pi/registry.rs:10-13,31,43,76,84` — Registry documented. Missing: `src/format.rs:5,14` — `escape_html` and `markdown_to_telegram_html` lack doc comments. `src/paths.rs:7-48` — All path helpers lack doc comments. |
| Recommendation | Add doc comments to `format.rs` functions (they're public). Add doc comments to `paths.rs` explaining what each path resolves to. Add usage examples to `PiSession::prompt_streaming` showing how to consume the channel. |

## 6. Minimalism

| Score | **A-** |
|-------|-------|
| Evidence | `src/pi/session.rs:82-117` — `PiSession` trait exposes exactly what's needed: metadata, stats, prompt (3 variants), abort, model switch, dispose. No unnecessary methods. `src/bot/state.rs:45-106` — `BotChatState` only exposes what the bot layer needs: status, busy check, begin/end processing, last_prompt. `src/pi/registry.rs:15-89` — `SessionRegistry` is minimal: new, get_or_create, remove, list. `src/config.rs:106-121` — `TelePiConfig` has all `pub` fields, exposing internals. `src/error.rs:5-38` — `TelePiError` has 11 variants covering distinct failure modes, none redundant. Minor concern: `src/pi/cli_session.rs:154-161` — `ModelInfo` is in `cli_session` module but referenced by `handler.rs` via full path `crate::pi::cli_session::ModelInfo`. |
| Recommendation | `TelePiConfig` fields should be non-pub with accessors if the config is treated as immutable after loading. Consider promoting `ModelInfo` to `pi/session.rs` or a shared types module since it's used across layers. `BotChatState::begin_switching/end_switching/begin_transcribing/end_transcribing` (`src/bot/state.rs:82-100`) are only used for `ChatStatus` variants but add API surface — consider a generic `set_status/remove_status` instead. |

---

## Summary

| Criterion | Score | Notes |
|-----------|-------|-------|
| Consistency | **B** | Good trait design; minor naming and error wrapping inconsistencies |
| Discoverability | **A** | Excellent doc coverage; self-documenting errors and commands |
| Backward Compatibility | **C** | No `#[non_exhaustive]` on key enums/structs; adding fields/variants is breaking |
| Input Validation | **B** | Config boundaries validated; some silent fallbacks need logging |
| Documentation | **B+** | Strong doc comments on traits and types; gaps in utility modules |
| Minimalism | **A-** | Clean trait surface; config fields over-exposed; minor cross-layer type placement |

## Top 3 Improvements

1. **Add `#[non_exhaustive]` to `PiEvent`, `TelePiError`, `ToolVerbosity`** — prevents downstream breakage when new variants are added.
2. **Make `TelePiConfig` fields private** with accessors, or add `#[non_exhaustive]` — the struct is the primary config boundary and should be forward-compatible.
3. **Promote `ModelInfo` to `pi/session.rs`** — it leaks `cli_session` internals into the bot layer via cross-module references.
