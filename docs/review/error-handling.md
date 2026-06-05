# Error Handling Review

> Reviewed: 2026-06-05
> Scope: `src/` (all modules), excluding `docs/remote/`

## Summary

TelePi's error handling is **solid for a small-to-medium project**. A well-designed `thiserror` enum, consistent `?` propagation, and user-facing error translation via `to_friendly_error()` show intentional design. The main weaknesses are inconsistent Result types at boundaries, a few `expect()`/`unwrap()` calls that could panic in edge-case environments, and error messages that leak Rust internals to Telegram users.

---

## 1. Error Propagation

| | |
|---|---|
| **Score** | **B** |
| **Evidence** | `src/error.rs:46` defines `pub type Result<T> = std::result::Result<T, TelePiError>`, used throughout `src/pi/cli_session.rs`, `src/config.rs`, `src/bot/handler.rs:323` (`process_prompt` returns `crate::error::Result<()>`). `?` propagation in `cli_session.rs:216,243,261,282,301,404`. |
| **Strengths** | Custom `Result` alias with domain error type. `#[from]` auto-conversions for `io::Error`, `reqwest::Error`, `serde_json::Error`, `anyhow::Error` (`error.rs:28-37`). `From<teloxide::RequestError>` manual impl (`error.rs:40-44`). |
| **Issues** | **Three Result types at boundaries**: `anyhow::Result` in `bot/mod.rs:run()`, `teloxide::ResponseResult<()>` in command dispatch (`bot/commands/mod.rs:63`), typed `crate::error::Result` elsewhere. Channel `tx.send(...).await.ok()` silently drops events if receiver is dropped (`cli_session.rs:328,332,340,345,427,435`). Cleanup ops like `tokio::fs::remove_file().await.ok()` and `transport::edit_text().await.ok()` swallow errors intentionally — acceptable for non-critical paths, but no logging. |
| **Recommendation** | Log channel send failures at `warn` level instead of silently `.ok()`. Document why cleanup errors are intentionally ignored. Consider unifying the Result type at entry points — at minimum, convert teloxide `ResponseResult` to `TelePiError` at the boundary. |

## 2. Error Types

| | |
|---|---|
| **Score** | **B+** |
| **Evidence** | `src/error.rs:5-38` — 11 variants covering all domain areas. `#[error("...")]` display strings on every variant. `to_friendly_error()` at `error.rs:49-65` maps each variant to a user-facing string. |
| **Strengths** | Clean 1:1 mapping between variants and domain areas. `Io`, `Http`, `Serde` carry structured inner errors via `#[from]`. `Other(#[from] anyhow::Error)` provides escape hatch. |
| **Issues** | `Telegram`, `PiSession`, `PiProcess`, `Voice`, `Install` variants hold `String` payloads — the original structured error is lost, replaced by formatted string. `From<teloxide::RequestError>` at `error.rs:40-44` does `format!("{err}")`, losing the request error enum. Five of eleven variants are stringly-typed. |
| **Recommendation** | For `PiProcess`, consider a sub-struct or at minimum include exit code: `PiProcess { message: String, exit_code: Option<i32> }`. For `Telegram`, keep the `RequestError` variant rather than stringifying. Low priority — the current approach is pragmatic and works. |

## 3. Edge Cases & Boundary Conditions

| | |
|---|---|
| **Score** | **B** |
| **Evidence** | Auth guard on every handler (`handler.rs:57,96,207`). Busy guard prevents concurrent operations (`handler.rs:66,102,213`). Empty content filtered in `prompt_inbox.rs:56,73`. Empty sessions list handled (`sessions.rs:33`). `.txt` extension filter on inbox entries (`prompt_inbox.rs:54`). |
| **Issues** | **Dead code logic bug** in `cli_session.rs:390-395`: the `if accumulated_text.is_empty()` guard around `if !accumulated_text.is_empty()` means the inner block is unreachable — the newline/newline append never happens. Not harmful (dead code), but indicates confused intent. **No unicode-safe slicing** in `handler.rs:162-163` — `&transcript.text[..100]` can panic on multi-byte UTF-8 characters at the boundary. `parse_model_list()` at `cli_session.rs:180` does `parts.len() >= 6` without checking individual part validity. `split_text()` at `transport.rs:108-130` splits by byte length, not char length — works for ASCII but could split mid-UTF-8 for non-ASCII Telegram messages. |
| **Recommendation** | Fix the `&transcript.text[..100]` slice to use `char_indices()` or `.chars().take(100).collect()`. Fix the dead-code logic bug in `cli_session.rs:390`. Add a UTF-8-safe boundary check in `split_text()`. |

## 4. Panic Safety

| | |
|---|---|
| **Score** | **B-** |
| **Evidence** | `paths.rs:8` — `dirs::home_dir().expect("could not determine home directory")` panics if home dir is missing. Called from `paths.rs:14,16,34,44,50,52` (6 call sites across config, install, tree). `format.rs:60,70` — `caps.get(1).or(caps.get(2)).unwrap()` inside regex replacements. `transport.rs:75` — `last_msg.expect("at least one chunk")`. `handler.rs:163` — `&transcript.text[..100]` can panic on UTF-8 boundary. |
| **Safe unwraps** | `format.rs:21,44,57,67,77,88` — `Regex::new(...).unwrap()` inside `LazyLock::new()`. These panic at startup if regex is invalid — acceptable (compile-time-like). `config.rs:325,331,360,373,383` — all inside `#[cfg(test)]`. `cli_session.rs:335,336,343,360-362,369-371` — `unwrap_or()`/`unwrap_or_else()` with sensible defaults. |
| **No `panic!()` calls** | Zero `panic!()`, `unimplemented!()`, or `todo!()` macros found in production code (6 `TODO` comments only). |
| **Recommendation** | Replace `home_dir().expect(...)` with a Result-returning function — this is the single highest-risk panic in the codebase since it runs in every code path that touches config, install, or tree. The `format.rs` unwraps inside regex callbacks are safe by construction but could use `.unwrap_or_default()` for defense-in-depth. |

## 5. User-Facing Errors

| | |
|---|---|
| **Score** | **B+** |
| **Evidence** | `to_friendly_error()` at `error.rs:49-65` maps every variant to a readable message. Used conceptually, though handlers often display raw errors directly. `handler.rs:83` sends `"❌ Error: {e}"` to Telegram. `handler.rs:138,144,150,258,264,270,286` all send `❌ {category}: {error}` messages. `sessions.rs:21,63` send `❌ Failed/Error` messages. |
| **Strengths** | Consistent `❌` prefix pattern. Error context preserved (category + message). Busy guard gives clear `"⏳ Still processing"` feedback. Unauthorized users get `"⛔ You are not authorized"`. |
| **Issues** | Handler errors at `handler.rs:83,182,308,484` display raw `TelePiError` `Display` output (`{e}`), which includes the variant prefix (e.g., "pi process error: pi process failed — check pi CLI output"). `to_friendly_error()` exists but is never called in any handler — it's dead code. The error messages leak Rust-level details like `reqwest::Error` Display strings and `io::Error` messages to Telegram users. |
| **Recommendation** | Use `to_friendly_error(&e)` in all handler error paths instead of `format!("❌ Error: {e}")`. This is the intended design that's not wired up. Low-effort, high-impact improvement. |

## 6. Recovery & Resilience

| | |
|---|---|
| **Score** | **B+** |
| **Evidence** | `transport.rs:14-37` — `with_retry()` implements 3-attempt exponential backoff (2s × attempt) for Telegram network errors only (API errors fail immediately). `bot/mod.rs` retries 409 Conflict up to 5× with 3s delay. `handler.rs:197,297` — temp files cleaned up via `remove_file().await.ok()` even after errors. `handler.rs:76` — `send_typing().await.ok()` continues if typing indicator fails. `handler.rs:440-465` — abort handler sends SIGTERM even if session retrieval fails. |
| **Strengths** | Graceful degradation on non-critical failures (typing indicators, message edits). Temp file cleanup runs in all code paths. Abort works even when session state is inconsistent. Busy guard prevents resource exhaustion from concurrent prompts. |
| **Issues** | No circuit breaker or rate limiting for the Pi CLI subprocess — if `pi` hangs, the chat is stuck until `/abort`. No timeout on `session.prompt_streaming()` itself (only the edit task has a 5s timeout at `handler.rs:429`). The `process_prompt` function at `handler.rs:317-437` has no overall timeout. |
| **Recommendation** | Add an overall timeout to `process_prompt()` (e.g., 5 minutes) to prevent indefinite hangs. Consider a watchdog timer that auto-aborts if no PiEvent received within N seconds. |

---

## Summary Table

| Criterion | Score | Key Finding |
|-----------|-------|-------------|
| Error Propagation | **B** | Solid `?` usage; inconsistent Result types at boundaries |
| Error Types | **B+** | Well-designed `thiserror` enum; some stringly-typed variants |
| Edge Cases | **B** | Good auth/busy guards; UTF-8 slicing bug; dead-code logic bug |
| Panic Safety | **B-** | `home_dir().expect()` can crash in constrained environments |
| User-Facing Errors | **B+** | `to_friendly_error()` designed but never called; raw errors leak |
| Recovery | **B+** | Good retry/cleanup; no overall timeout on prompt processing |

**Overall: B**

## Top 5 Quick Wins

1. **Wire up `to_friendly_error()`** in all handler error paths (`handler.rs:83,182,308,484`) — 15 min, high impact
2. **Fix UTF-8 panic** in `handler.rs:163` (`&transcript.text[..100]`) — 5 min, prevents production panic
3. **Replace `home_dir().expect()`** in `paths.rs:8` with a fallible function — 30 min, prevents startup crash
4. **Add overall timeout** to `process_prompt()` — 30 min, prevents indefinite hangs
5. **Fix dead-code logic bug** in `cli_session.rs:390-395` — 5 min, code hygiene
