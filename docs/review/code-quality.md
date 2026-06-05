# Code Quality Review — TelePi

> Reviewed: 2026-06-05
> Scope: `src/` (Rust source), excludes `docs/remote/` and external projects.

---

## 1. Naming

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **A** | Consistent Rust conventions throughout: `snake_case` for functions/variables, `PascalCase` for types. Domain names are clear: `process_prompt`, `chat_key_to_context`, `markdown_to_telegram_html`, `SessionContext`, `BotChatState`. Serde enums use `kebab-case` (`ToolVerbosity`). BotCommands use `lowercase`. Import order follows `std → external → crate:: → self::` (documented in `conventions.md`). | No changes needed. Maintain current naming style. |

---

## 2. File Organization

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **A** | Clean 3-layer module hierarchy: `cli.rs` → `bot/` → `pi/`. Largest file is `pi/cli_session.rs` (547 lines) — acceptable for a session manager with JSON protocol. `handler.rs` (490 lines) is the only borderline case. Support modules are single-purpose: `format.rs` (146 lines), `paths.rs` (54 lines), `error.rs` (65 lines). `mod.rs` files are minimal re-exports (convention documented). No flat-file sprawl. | Consider splitting `handler.rs` into `handler/mod.rs` + `handler/text.rs` + `handler/media.rs` if it grows further. Currently acceptable. |

---

## 3. Code Duplication

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **C+** | **Telegram file download** is copy-pasted across `voice_handler` (lines 124–152) and `photo_handler` (lines 244–272): identical `temp_dir` creation, `bot.get_file()`, `reqwest::Client::new()`, nested match on response bytes, `tokio::fs::write`. Only the filename prefix and error message differ. **Auth + busy guard** boilerplate (check `is_allowed_user`, check `is_busy`, send "⏳ Still processing") is repeated in every handler: `text_handler:50–60`, `voice_handler:92–106`, `photo_handler:203–217`, `abort_handler:440+`, `retry_handler:468+`. **Error messaging** `format!("❌ Session error: {e}")` appears in 5 different command files with nearly identical match-err-send-return blocks. | Extract `download_telegram_file(bot, config, file_id, temp_name) -> Result<PathBuf>` to `transport.rs`. Extract `guard_user_and_busy(bot, msg, state) -> Option<(HandlerState, ChatKey)>` helper. Extract `session_error(bot, chat_id, e)` macro or helper for the repeated error-send-return pattern. |

---

## 4. Complexity

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **B** | `process_prompt` (`handler.rs:317–491`) is ~175 lines containing a spawned task with a 9-variant `PiEvent` match, debounced edit logic, tool output accumulation, and final message splitting — high cognitive load. `markdown_to_telegram_html` (`format.rs:14–106`) is ~90 lines of sequential regex passes with placeholder extraction — linear and readable, but a regex failure in any step silently propagates. `cli_session.rs` `prompt_streaming` (lines 250–420) handles process spawning, stdout line parsing, error recovery, and event translation — ~170 lines. These three functions contain the bulk of the project's complexity. | Extract the `tokio::spawn` edit loop from `process_prompt` into `fn spawn_edit_loop(bot, chat_id, msg_id, rx) -> JoinHandle`. Extract JSON line parsing from `prompt_streaming` into a dedicated `parse_json_line(line) -> Option<PiEvent>` function. No structural issues — just long functions that would benefit from decomposition. |

---

## 5. Dead Code

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **C** | **Unwired command stubs**: `cmd_branch` and `cmd_label` in `commands/tree.rs:61–111` are fully implemented but not wired into the `Command` enum — unreachable from the bot. Both contain `// TODO: Implement actual…` placeholders. **`#[allow(dead_code)]` on 7 types** in `pi/cli_session.rs`: `CliSession` struct (line 19), `JsonEvent` enum (line 44), `JsonMessage` (line 68), `JsonUsage` (line 82), `JsonCost` (line 94), `AssistantMessageEvent` (line 100), `ToolCallInfo` (line 146). These are serde deserialization targets — the suppression is necessary for serde's field consumption, but the blanket `#[allow(dead_code)]` masks genuine unused fields. **Stub backends**: `transcribe_parakeet` and `transcribe_sherpa` in `voice/mod.rs:74–82` are placeholder implementations that return errors. **Unused variables**: `_entry_id`, `_label` in `commands/tree.rs:74,103–104` — parsed but never used. | Wire `cmd_branch`/`cmd_label` into the `Command` enum or delete them. Replace blanket `#[allow(dead_code)]` with targeted `#[serde(default)]` on specific fields where possible. Add `#[cfg(feature = "parakeet")]` / `#[cfg(feature = "sherpa")]` gating on the stub backends so they're not compiled unless the feature is enabled. |

---

## 6. Anti-Patterns

| Score | Evidence | Recommendation |
|-------|----------|----------------|
| **B-** | **6 TODOs** across the codebase (0 FIXME/HACK/XXX/DEPRECATED): `cli_session.rs:466` (stats), `tree.rs:76,106` (branch/label), `voice/mod.rs:75,81` (backends), `install/mod.rs:30` (service running check). Density is low but severity varies — the `install/mod.rs` TODO means `ServiceStatus` always reports "not running" regardless of actual state. **`unwrap()` in static context**: 8 calls in `format.rs` inside `LazyLock::new(|| Regex::new(…).unwrap())` — acceptable for compile-time-constant regexes. **`expect()` in `home_dir()`**: `paths.rs:8` panics if home directory can't be determined — reasonable for a CLI tool. **`unwrap_or_else(|| PathBuf::from("/tmp"))`** in `cli_session.rs:212` — silently falls back to `/tmp` if `data_dir()` is unavailable, which could cause session data loss. **Error type inconsistency** (documented in `conventions.md`): `anyhow::Result` in `bot/mod.rs:run()`, `teloxide::ResponseResult<()>` in command dispatch, typed `crate::error::Result` everywhere else — 3 different result types in one crate. | Prioritize fixing `install/mod.rs:30` — the service status check is user-facing and always wrong. Standardize on typed `crate::error::Result` where possible; use `anyhow` only at the top-level entrypoint. Log a warning when `data_dir()` falls back to `/tmp`. |

---

## Summary

| Criterion | Score | Key Issue |
|-----------|-------|-----------|
| Naming | **A** | Clean, consistent, idiomatic Rust |
| File Organization | **A** | Good module hierarchy, reasonable file sizes |
| Code Duplication | **C+** | Telegram download + auth guard boilerplate copy-pasted across 3+ handlers |
| Complexity | **B** | 3 functions at 150+ lines; need extraction of sub-steps |
| Dead Code | **C** | Unwired stubs, 7x `#[allow(dead_code)]`, placeholder backends |
| Anti-Patterns | **B-** | 6 TODOs, error type inconsistency, one always-wrong status check |

**Overall: B**

The codebase follows Rust conventions well and has a clean architecture. The main improvement areas are (1) extracting shared Telegram file download and auth guard patterns into reusable helpers, (2) decomposing the three 150+ line functions, and (3) cleaning up dead code and unwired stubs. No critical safety issues — `unwrap()`/`panic!()` usage is confined to valid contexts (static regex, home dir).
